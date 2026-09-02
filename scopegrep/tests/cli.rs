//! バイナリを実際に起動して、出力と終了コードを固定する統合テスト（QLT-007）。
//!
//! 🔴 **出力の完全一致を見る。** README と設計メモが約束している形が、
//! 実際に出ている形であることを機械で確かめるのがここの仕事である。
//! 「だいたい合っている」を許すと、地雷2（動かない例が載っている public リポ）に戻る。
//!
//! 🔑 clippy がテストで `unwrap` / `expect` を許すのは **`#[test]` 関数の中だけ**である
//! （実測）。だから補助関数は落ちない形で書き、落ちる判断は必ずテスト本体に置く。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// core と同じ fixture を CLI からも見る。**判定の基準は1つにする。**
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../scopegrep-core/testdata/workflow-with-comment.yml"
);

/// 読める部分集合の外にある fixture（アンカー・5行目）。
const ANCHOR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/testdata/unsupported-anchor.yml"
);

/// 走査順の判定に使う YAML。どのファイルにも1件だけヒットがある。
const SEED: &str = "steps:\n  - name: A\n    run: target\n";

/// バイナリを起動する。起動できないことの判断はテスト本体に任せる。
fn spawn(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_scopegrep"))
        .args(arguments)
        .output()
}

/// 標準出力を文字列にする。
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// 標準エラーを文字列にする。
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// 走査順の判定に使うディレクトリを作る。
///
/// `.git/` と `.txt` は**読まれてはならない**ものとして置く。
fn seed_tree(root: &Path) -> io::Result<()> {
    fs::create_dir_all(root.join("z"))?;
    fs::create_dir_all(root.join(".git"))?;
    fs::write(root.join("a.yaml"), SEED)?;
    fs::write(root.join("m.yml"), SEED)?;
    fs::write(root.join("z/inner.yml"), SEED)?;
    fs::write(root.join("b.txt"), SEED)?;
    fs::write(root.join(".git/hidden.yml"), SEED)?;
    Ok(())
}

/// 一意な一時ディレクトリのパス。
fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("scopegrep-cli-{}-{name}", std::process::id()))
}

/// `<dir>/<file>:3: steps[0] "A" .run = target` の1行。
fn seeded_line(root: &Path, relative: &str) -> String {
    let mut line = root.join(relative).display().to_string();
    line.push_str(":3: steps[0] \"A\" .run = target\n");
    line
}

// ── 1. 人向け出力の完全一致 ─────────────────────────────────────────────────

#[test]
fn a_human_run_prints_the_scope_of_every_hit() {
    let output = spawn(&["cancelled()", FIXTURE]).expect("バイナリを起動できるはず");
    let expected = FIXTURE.to_owned()
        + ":33: jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if = ${{ !cancelled() }}\n"
        + FIXTURE
        + ":46: jobs.e2e.steps[2] \"Upload Playwright report\" .if = ${{ !cancelled() }}\n";
    assert_eq!(stdout(&output), expected);
    assert_eq!(stderr(&output), "");
    assert_eq!(output.status.code(), Some(0_i32));
}

/// 🔴 コメント内の `cancelled()` は3件ある。それが出ないことがこの道具の存在理由である。
#[test]
fn comments_never_become_hits() {
    let output = spawn(&["cancelled()", FIXTURE]).expect("バイナリを起動できるはず");
    assert_eq!(stdout(&output).lines().count(), 2);
}

// ── 2. JSON 出力の完全一致 ──────────────────────────────────────────────────

#[test]
fn a_json_run_prints_seven_keys_in_a_fixed_order() {
    let output = spawn(&["--json", "cancelled()", FIXTURE]).expect("バイナリを起動できるはず");
    let expected = "{\"file\":\"".to_owned()
        + FIXTURE
        + "\",\"line\":33,\"column\":18,\"pointer\":\"/jobs/frontend-check/steps/3/if\","
        + "\"path\":\"jobs.frontend-check.steps[3] \\\"Audit (fail on high/critical)\\\" .if\","
        + "\"label\":\"Audit (fail on high/critical)\",\"value\":\"${{ !cancelled() }}\"}\n"
        + "{\"file\":\""
        + FIXTURE
        + "\",\"line\":46,\"column\":18,\"pointer\":\"/jobs/e2e/steps/2/if\","
        + "\"path\":\"jobs.e2e.steps[2] \\\"Upload Playwright report\\\" .if\","
        + "\"label\":\"Upload Playwright report\",\"value\":\"${{ !cancelled() }}\"}\n";
    assert_eq!(stdout(&output), expected);
    assert_eq!(output.status.code(), Some(0_i32));
}

// ── 3. ヒット無し ───────────────────────────────────────────────────────────

#[test]
fn no_hit_prints_nothing_and_exits_one() {
    let output = spawn(&["no-such-needle", FIXTURE]).expect("バイナリを起動できるはず");
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
    assert_eq!(output.status.code(), Some(1_i32));
}

// ── 4. 部分集合の外 ─────────────────────────────────────────────────────────

/// 🔴 読めないファイルがあっても、他のファイルの結果は出る。**それでも 2 で終わる。**
/// 「一部しか見ていない結果」を成功と呼ばないのが、この道具が生まれた事故への答えである。
#[test]
fn an_unreadable_file_is_reported_by_line_and_still_exits_two() {
    let output = spawn(&["cancelled()", ANCHOR, FIXTURE]).expect("バイナリを起動できるはず");
    let expected_error =
        "scopegrep: ".to_owned() + ANCHOR + ":5: アンカー（&name） は読めない構文である\n";
    assert_eq!(stderr(&output), expected_error);
    assert_eq!(stdout(&output).lines().count(), 2);
    assert_eq!(output.status.code(), Some(2_i32));
}

// ── 5. 存在しないパス ───────────────────────────────────────────────────────

#[test]
fn a_missing_path_is_reported_and_exits_two() {
    let output = spawn(&["x", "no/such/file.yml"]).expect("バイナリを起動できるはず");
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).starts_with("scopegrep: no/such/file.yml: "),
        "実際の出力: {}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(2_i32));
}

// ── 6. 使い方・版 ───────────────────────────────────────────────────────────

#[test]
fn bad_arguments_print_the_usage_and_exit_two() {
    for arguments in [vec![], vec!["cancelled()"]] {
        let output = spawn(&arguments).expect("バイナリを起動できるはず");
        assert_eq!(
            stderr(&output),
            "scopegrep: usage: scopegrep [--json] <needle> <path>...\n"
        );
        assert_eq!(stdout(&output), "");
        assert_eq!(output.status.code(), Some(2_i32));
    }
}

#[test]
fn help_and_version_succeed_on_stdout() {
    for flag in ["--help", "-h"] {
        let output = spawn(&[flag]).expect("バイナリを起動できるはず");
        assert!(stdout(&output).starts_with("scopegrep — "));
        assert_eq!(stderr(&output), "");
        assert_eq!(output.status.code(), Some(0_i32));
    }
    for flag in ["--version", "-V"] {
        let output = spawn(&[flag]).expect("バイナリを起動できるはず");
        assert_eq!(
            stdout(&output),
            format!("scopegrep {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(output.status.code(), Some(0_i32));
    }
}

// ── 7. ディレクトリ走査 ─────────────────────────────────────────────────────

/// `.yml` / `.yaml` は読む・`.txt` は読まない・`.git/` は掘らない・並びはバイト順。
#[test]
fn a_directory_is_walked_in_byte_order_without_git_or_other_extensions() {
    let root = temporary("walk");
    seed_tree(&root).expect("一時ディレクトリを作れるはず");
    let path = root.display().to_string();

    let output = spawn(&["target", &path]).expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("一時ディレクトリを片付けられるはず");

    let expected = seeded_line(&root, "a.yaml")
        + &seeded_line(&root, "m.yml")
        + &seeded_line(&root, "z/inner.yml");
    assert_eq!(stdout(&output), expected);
    assert_eq!(stderr(&output), "");
    assert_eq!(output.status.code(), Some(0_i32));
}

/// ファイルとして名指しされたパスは**拡張子を問わず読む**。
#[test]
fn a_named_file_is_read_whatever_its_extension() {
    let root = temporary("named");
    seed_tree(&root).expect("一時ディレクトリを作れるはず");
    let path = root.join("b.txt").display().to_string();

    let output = spawn(&["target", &path]).expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("一時ディレクトリを片付けられるはず");

    assert_eq!(stdout(&output), seeded_line(&root, "b.txt"));
    assert_eq!(output.status.code(), Some(0_i32));
}
