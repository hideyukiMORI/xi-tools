//! README の `console` ブロックが、**実際の出力と一致する**ことを機械で確かめる。
//!
//! 🔴 README はこのリポジトリの成果物の本体である（CLAUDE.md）。
//! 動かない例が載っている public リポは、道具の中身を読まれる前に評価が終わる。
//! だからここでは README を読むのではなく、**書いてあるコマンドを実行して比較する**。
//!
//! 🔑 例が README から消えても気づけるように、`$ scopegrep` の例が
//! 1つも無ければ失敗する。「検査が空振りしても緑になる」形にしない（QLT-007）。
//!
//! コマンドはリポジトリのルートを cwd にして走らせる。README の例が
//! ルートから打った形で書かれているからで、cwd を変えると例の意味が変わる。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// README の `console` ブロックから取り出した例1件。
///
/// 🔴 フィールドは非公開である（RS-008）。行番号は README の行で、失敗したときに
/// 「README のどこを直すか」を言うために持つ。
#[derive(Debug, Clone)]
struct Example {
    line: usize,
    command: String,
    expected: String,
}

impl Example {
    /// `$ ` 行から始める。出力はこの後 [`Example::push_output`] で積む。
    fn new(line: usize, command: &str) -> Self {
        Self {
            line,
            command: command.trim_end().to_owned(),
            expected: String::new(),
        }
    }

    /// 期待する出力を1行足す。README の改行をそのまま持つ。
    fn push_output(&mut self, line: &str) {
        self.expected.push_str(line);
        self.expected.push('\n');
    }

    /// README の何行目に書かれた例か。
    fn line(&self) -> usize {
        self.line
    }

    /// `$ ` を除いたコマンド行。
    fn command(&self) -> &str {
        &self.command
    }

    /// 続く行から組み立てた、期待する標準出力。
    fn expected(&self) -> &str {
        &self.expected
    }
}

/// リポジトリのルート。`scopegrep/` の1つ上。
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(".."))
        .to_path_buf()
}

/// README の本文。読めなければ空文字列になり、例が 0 件として失敗する。
fn readme() -> String {
    std::fs::read_to_string(repository_root().join("README.md")).unwrap_or_default()
}

/// ```` ```console ```` ブロックの `$ …` 行と、それに続く出力を取り出す。
fn examples(text: &str) -> Vec<Example> {
    let mut found: Vec<Example> = Vec::new();
    let mut inside = false;
    for (index, line) in text.lines().enumerate() {
        if line.starts_with("```") {
            inside = line.trim_end() == "```console";
        } else if inside {
            match line.strip_prefix("$ ") {
                Some(command) => found.push(Example::new(index.saturating_add(1_usize), command)),
                None => append_output(&mut found, line),
            }
        }
    }
    found
}

/// 直前の例に出力行を足す。`$` 行の前に文字があれば無視する。
fn append_output(found: &mut [Example], line: &str) {
    if let Some(current) = found.last_mut() {
        current.push_output(line);
    }
}

/// コマンド行を引数に分ける。`'…'` のクォートは1枚剥がす。
///
/// 🔑 シェルの真似はしない。README に書いてよいのは
/// 「空白区切り＋シングルクォート」だけで、それ以外を書きたくなったら
/// **README ではなくテストの側を直すべきかを先に考える**。
fn split_arguments(command: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut written = false;
    for character in command.chars() {
        match character {
            '\'' => {
                quoted = !quoted;
                written = true;
            }
            ' ' if !quoted => push_token(&mut found, &mut current, &mut written),
            plain => current.push(plain),
        }
    }
    push_token(&mut found, &mut current, &mut written);
    found
}

/// 組み立て中の引数を1つ確定させる。空なら何もしない。
///
/// 🔑 `''` だけは例外で、**空のまま1つの引数になる**。`--scope` で所属だけを
/// 指定するとき、needle は空文字列になる（`scopegrep --scope '/a/b' ''`）ので、
/// ここで落とすと README の例と実際の起動が食い違う。
fn push_token(found: &mut Vec<String>, current: &mut String, written: &mut bool) {
    if !current.is_empty() || *written {
        found.push(core::mem::take(current));
    }
    *written = false;
}

/// README に書いてよいコマンドは2つだけ。
///
/// 🔴 知らないコマンドを**黙って飛ばさない**。飛ばすと、検証されない例が
/// README に残っていることに誰も気づけない。
fn executable(program: &str) -> Option<&'static str> {
    match program {
        "scopegrep" => Some(env!("CARGO_BIN_EXE_scopegrep")),
        "grep" => Some("grep"),
        _ => None,
    }
}

/// 例1件を、リポジトリのルートを cwd にして実行する。
fn run(root: &Path, arguments: &[String]) -> Option<Output> {
    let (program, rest) = arguments.split_first()?;
    let path = executable(program)?;
    Command::new(path)
        .args(rest)
        .current_dir(root)
        .output()
        .ok()
}

// ── 1. README の例は、実行した出力と完全一致する ───────────────────────────

#[test]
fn every_console_example_matches_the_real_output() {
    let root = repository_root();
    let text = readme();
    let found = examples(&text);
    assert!(
        !found.is_empty(),
        "README に console ブロックの例が1つも無い"
    );

    for example in &found {
        let arguments = split_arguments(example.command());
        let Some(output) = run(&root, &arguments) else {
            panic!(
                "README {}行目のコマンドを実行できない: $ {}",
                example.line(),
                example.command()
            );
        };
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            example.expected(),
            "README {}行目の例が実際の出力と違う: $ {}",
            example.line(),
            example.command()
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "",
            "README {}行目の例が標準エラーに書いている",
            example.line()
        );
    }
}

// ── 2. 例そのものが消えたら気づく ───────────────────────────────────────────

/// 🔴 例が README から消えると、上のテストは「全ての例が一致した」と言って緑になる。
/// **空振りしても緑になる検査は、検査ではない**（QLT-007）。
#[test]
fn the_readme_keeps_at_least_one_scopegrep_example() {
    let text = readme();
    let count = examples(&text)
        .iter()
        .filter(|example| example.command().starts_with("scopegrep "))
        .count();
    assert!(
        count >= 1_usize,
        "README から `$ scopegrep` の例が消えている（実際: {count} 件）"
    );
}

// ── 3. 例の読み方そのもの ───────────────────────────────────────────────────

#[test]
fn a_quoted_needle_stays_one_argument() {
    assert_eq!(
        split_arguments("scopegrep 'cancelled() and more' testdata/"),
        vec![
            "scopegrep".to_owned(),
            "cancelled() and more".to_owned(),
            "testdata/".to_owned(),
        ]
    );
}

/// 🔴 `''` は**空の引数1つ**である。落とすと `--scope '/a' ''` の例で
/// パスが needle として読まれ、README の例と実際の起動が食い違う。
#[test]
fn an_explicitly_empty_argument_survives() {
    assert_eq!(
        split_arguments("scopegrep --scope '/jobs/*/steps' '' testdata/"),
        vec![
            "scopegrep".to_owned(),
            "--scope".to_owned(),
            "/jobs/*/steps".to_owned(),
            String::new(),
            "testdata/".to_owned(),
        ]
    );
}

/// ```` ```bash ```` のブロックは実行しない。README の「開発」節の `make check` を
/// 走らせてしまうと、テストが自分自身を呼ぶ。
#[test]
fn only_console_blocks_are_executed() {
    let text = "```bash\n$ make check\n```\n```console\n$ grep -n x a\n1:x\n```\n";
    let found = examples(text);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found.first().map(Example::command),
        Some("grep -n x a"),
        "console ブロックの例だけを拾う"
    );
    assert_eq!(found.first().map(Example::expected), Some("1:x\n"));
}
