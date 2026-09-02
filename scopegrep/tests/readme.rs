//! README の `console` ブロックが、**実際の出力と一致する**ことを機械で確かめる。
//!
//! 🔴 README はこのリポジトリの成果物の本体である（CLAUDE.md）。
//! 動かない例が載っている public リポは、道具の中身を読まれる前に評価が終わる。
//! だからここでは README を読むのではなく、**書いてあるコマンドを実行して比較する**。
//!
//! 🔑 例が README から消えても気づけるように、`$ scopegrep` の例が
//! 1つも無ければ失敗する。「検査が空振りしても緑になる」形にしない（QLT-007）。
//!
//! 🔴 **飛ばしてよい例は1種類だけである**——正規表現なしでビルドされた binary での
//! `-e` の例（ADR 0002）。`make check` は feature 付きでも走るので、
//! **どの例も、いずれかの構成では必ず実行される**。飛ばした結果 1 件も実行されなければ失敗する。
//!
//! 🔴 **照合する README は1つではない**。`README.md`（英語）と `README.ja.md`（日本語）は
//! 同じ `console` ブロックを持つので、**両方に同じ検査をかける**。片方だけ見ると、
//! もう片方に「動かない例」が残っていても緑になる。
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

/// 照合する README。**英語版と日本語版の両方**を同じ検査にかける。
///
/// 🔴 ここから名前を減らさないこと。減らした README は誰にも照合されなくなる。
const READMES: [&str; 2] = ["README.md", "README.ja.md"];

/// README の本文。読めなければ空文字列になり、例が 0 件として失敗する。
fn readme(name: &str) -> String {
    std::fs::read_to_string(repository_root().join(name)).unwrap_or_default()
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

/// この構成では実行できない例か。
///
/// 🔴 **飛ばしてよい理由は1つだけである**——正規表現なしでビルドされた binary で
/// `-e` の例を実行しても、それは README の誤りではなく構成の話だからである。
/// `make check` は feature 付きでも走るので、**この例が検証されない構成は無い**。
/// 飛ばした結果「1つも検証されなかった」なら、下のテストが落ちる。
///
/// 🔑 `--` より後ろの `-e` は位置引数だが、README にそう書く例は無い
/// （書きたくなったら、README ではなくここを直すべきかを先に考える）。
fn skipped(arguments: &[String]) -> bool {
    cfg!(not(feature = "regex"))
        && arguments
            .iter()
            .any(|word| word == "-e" || word == "--regex")
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
    for name in READMES {
        verify_examples(name);
    }
}

/// README 1ファイル分の例を、すべて実行して照合する。
fn verify_examples(name: &str) {
    let root = repository_root();
    let text = readme(name);
    let found = examples(&text);
    assert!(
        !found.is_empty(),
        "{name} に console ブロックの例が1つも無い"
    );

    let mut verified = 0_usize;
    for example in &found {
        verified = verified.saturating_add(verify_example(&root, name, example));
    }

    // 🔴 飛ばした結果として1件も実行されなかったなら、それは緑ではない（QLT-007）。
    assert!(
        verified >= 1_usize,
        "この構成の {name} で検証できた例が1つも無い（例 {} 件すべて飛ばされた）",
        found.len()
    );
}

/// 例1件を照合する。実行したなら 1、この構成で飛ばしたなら 0 を返す。
///
/// 🔑 実行できなかったことは `assert!` で落とす。`panic!` は本番コードでは
/// 禁止されており（`clippy::panic` = forbid）、**この関数は `#[test]` ではない**ので
/// テストの免除も効かない。免除を広げるのではなく、書き方を変える。
fn verify_example(root: &Path, name: &str, example: &Example) -> usize {
    let arguments = split_arguments(example.command());
    if skipped(&arguments) {
        return 0_usize;
    }
    let executed = run(root, &arguments);
    assert!(
        executed.is_some(),
        "{name} {}行目のコマンドを実行できない: $ {}",
        example.line(),
        example.command()
    );
    executed.map_or(0_usize, |output| {
        compare_output(name, example, &output);
        1_usize
    })
}

/// 実行結果を README の記述と突き合わせる。標準エラーに何か書いていても落とす。
fn compare_output(name: &str, example: &Example, output: &Output) {
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        example.expected(),
        "{name} {}行目の例が実際の出力と違う: $ {}",
        example.line(),
        example.command()
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "",
        "{name} {}行目の例が標準エラーに書いている",
        example.line()
    );
}

// ── 2. 例そのものが消えたら気づく ───────────────────────────────────────────

/// 🔴 例が README から消えると、上のテストは「全ての例が一致した」と言って緑になる。
/// **空振りしても緑になる検査は、検査ではない**（QLT-007）。
///
/// 🔴 **どちらか一方で 0 件でも落とす。** 英語版と日本語版は同じ例を持つので、
/// 片方から例が消えた状態は「翻訳が片側だけ動いた」ことそのものである。
#[test]
fn each_readme_keeps_at_least_one_scopegrep_example() {
    for name in READMES {
        let text = readme(name);
        let count = examples(&text)
            .iter()
            .filter(|example| example.command().starts_with("scopegrep "))
            .count();
        assert!(
            count >= 1_usize,
            "{name} から `$ scopegrep` の例が消えている（実際: {count} 件）"
        );
    }
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

/// 🔴 飛ばしてよいのは「この構成では実行できない例」だけである。
/// ここが緩むと、検証されない例が README に残っていることに誰も気づけなくなる。
#[test]
fn only_regex_examples_are_skipped_and_only_without_the_feature() {
    let plain = split_arguments("scopegrep 'cancelled()' scopegrep-core/testdata/");
    let expression = split_arguments("scopegrep -e 'cancel+ed' scopegrep-core/testdata/");
    assert!(!skipped(&plain), "正規表現でない例は構成によらず実行する");
    assert_eq!(
        skipped(&expression),
        cfg!(not(feature = "regex")),
        "-e の例は、正規表現なしでビルドしたときだけ飛ばす"
    );
}
