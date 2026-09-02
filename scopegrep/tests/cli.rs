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

/// v1.1 で読めるようになった3構文（複数行フロー・タグ・要素のフローマッピング）を
/// 1ファイルに集めた fixture。**架空の手書きデータ**である。
const COMPOSE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/compose-like.yaml");

/// 走査順の判定に使う YAML。どのファイルにも1件だけヒットがある。
const SEED: &str = "steps:\n  - name: A\n    run: target\n";

/// バイナリを起動する。起動できないことの判断はテスト本体に任せる。
fn spawn(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_scopegrep"))
        .args(arguments)
        .output()
}

/// `root` を cwd にしてバイナリを起動する。
///
/// 🔑 パスを省略したときの表示（`./` を付けない）は cwd を変えないと確かめられない。
fn spawn_in(root: &Path, arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_scopegrep"))
        .args(arguments)
        .current_dir(root)
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

/// 走査から外すディレクトリ。`walk.rs` の定数と同じ並びである。
const SKIPPED: [&str; 5] = [".git", "node_modules", "vendor", "target", ".venv"];

/// 走査順の判定に使うディレクトリを作る。
///
/// `.txt` と、依存ディレクトリ（`SKIPPED`）に置いた `.yml` は
/// **読まれてはならない**ものとして置く。
fn seed_tree(root: &Path) -> io::Result<()> {
    fs::create_dir_all(root.join("z"))?;
    fs::write(root.join("a.yaml"), SEED)?;
    fs::write(root.join("m.yml"), SEED)?;
    fs::write(root.join("z/inner.yml"), SEED)?;
    fs::write(root.join("b.txt"), SEED)?;
    for directory in SKIPPED {
        fs::create_dir_all(root.join(directory))?;
        fs::write(root.join(directory).join("hidden.yml"), SEED)?;
    }
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

/// 🔴 コメント内の `cancelled()` は3件ある。既定でそれが出ないことがこの道具の存在理由である。
#[test]
fn comments_never_become_hits() {
    let output = spawn(&["cancelled()", FIXTURE]).expect("バイナリを起動できるはず");
    assert_eq!(stdout(&output).lines().count(), 2);
}

// ── 1b. `--comments` は同じ5行を種別付きで返す ─────────────────────────────

/// 🔴 `grep -n` が返す5行と**同じ5行**が、値とコメントに分かれて返る。
/// 偽陽性を消すのではなく、**別枠にする**のがこの旗の意味である。
#[test]
fn comments_are_returned_as_comments_when_asked() {
    let output = spawn(&["--comments", "cancelled()", FIXTURE]).expect("バイナリを起動できるはず");
    let expected = FIXTURE.to_owned()
        + ":4: #comment = #    候補パーサは、下の3つの `cancelled()` を **別物として区別できなければならない**。\n"
        + FIXTURE
        + ":29: jobs.frontend-check.steps #comment = # 1) 散文。ここに書かれた cancelled() は設定値ではない。\n"
        + FIXTURE
        + ":30: jobs.frontend-check.steps #comment = #    !cancelled() を使う理由を説明しているだけで、実行条件ではない。\n"
        + FIXTURE
        + ":33: jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if = ${{ !cancelled() }}\n"
        + FIXTURE
        + ":46: jobs.e2e.steps[2] \"Upload Playwright report\" .if = ${{ !cancelled() }}\n";
    assert_eq!(stdout(&output), expected);
    assert_eq!(stderr(&output), "");
    // コメントもヒットである。ヒットがあるので 0 で終わる。
    assert_eq!(output.status.code(), Some(0_i32));
}

/// 🔴 旗を付けない既定の出力は**1文字も変わらない**。
#[test]
fn the_flag_does_not_change_the_default_output() {
    let plain = spawn(&["cancelled()", FIXTURE]).expect("バイナリを起動できるはず");
    let expected = FIXTURE.to_owned()
        + ":33: jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if = ${{ !cancelled() }}\n"
        + FIXTURE
        + ":46: jobs.e2e.steps[2] \"Upload Playwright report\" .if = ${{ !cancelled() }}\n";
    assert_eq!(stdout(&plain), expected);
}

/// コメントしか当たらない語でも、ヒットはヒットである（終了コード 0）。
#[test]
fn a_comment_only_match_still_exits_zero() {
    let bare = spawn(&["1) 散文", FIXTURE]).expect("バイナリを起動できるはず");
    assert_eq!(stdout(&bare), "");
    assert_eq!(bare.status.code(), Some(1_i32));

    let asked = spawn(&["--comments", "1) 散文", FIXTURE]).expect("バイナリを起動できるはず");
    assert_eq!(stdout(&asked).lines().count(), 1);
    assert_eq!(asked.status.code(), Some(0_i32));
}

// ── 1c. v1.1 の3構文（実ファイル計測の 18 件が根拠）──────────────────────

/// `<file>:<line>: ` で始まる1行を組み立てる。
fn compose_line(line: u32, tail: &str) -> String {
    format!("{COMPOSE}:{line}: {tail}\n")
}

/// 🔴 複数行にまたがるフロー記法。**各行が同じパスの別のスカラー行**として返る。
/// `[` が次の行に来る形（11 件）と、行内で開いて次の行で閉じる形（3 件）の両方。
#[test]
fn a_multi_line_flow_reports_the_line_that_actually_matched() {
    let output = spawn(&["8080", COMPOSE]).expect("バイナリを起動できるはず");
    let expected = compose_line(
        11,
        "services.api.healthcheck.test = \"curl -f http://localhost:8080/healthz || exit 1\"",
    ) + &compose_line(
        15,
        "services.api.command = [\"serve\", \"--port\", \"8080\",",
    );
    assert_eq!(stdout(&output), expected);
    assert_eq!(stderr(&output), "");
    assert_eq!(output.status.code(), Some(0_i32));
}

/// タグ（`!override` / `!!str`）は読み飛ばし、その後ろの値と入れ子を通常どおり読む。
#[test]
fn a_tagged_value_keeps_its_scope() {
    let output = spawn(&["5432", COMPOSE]).expect("バイナリを起動できるはず");
    let expected = compose_line(
        18,
        "services.api.environment.DATABASE_URL = postgres://api:example@db:5432/api",
    ) + &compose_line(24, "services.db.ports[0][0] = '15432:5432'");
    assert_eq!(stdout(&output), expected);
    assert_eq!(output.status.code(), Some(0_i32));
}

/// 🔴 v1 は `- { $ref: '…' }` の `{ $ref` をキーと誤読して「余分な文字」にしていた。
#[test]
fn a_flow_mapping_element_is_one_scalar() {
    let output = spawn(&["$ref", COMPOSE]).expect("バイナリを起動できるはず");
    let expected = compose_line(
        29,
        "parameters[0] = { $ref: '#/components/parameters/IdPath' }",
    ) + &compose_line(
        30,
        "parameters[1] = { $ref: '#/components/parameters/PageQuery' }",
    );
    assert_eq!(stdout(&output), expected);
    assert_eq!(output.status.code(), Some(0_i32));
}

/// 🔑 フローの行と、フローについて書かれたコメントは**別枠**である。
/// 括弧の中の `#` は値の一部なので、コメントとしては返らない。
#[test]
fn a_comment_about_a_flow_is_not_one_of_its_lines() {
    let output = spawn(&["--comments", "[", COMPOSE]).expect("バイナリを起動できるはず");
    let expected = compose_line(
        7,
        "services.api.healthcheck #comment = # `[` が次の行に来る形。実ファイル計測ではこれが 11 件で最多だった。",
    ) + &compose_line(9, "services.api.healthcheck.test = [")
        + &compose_line(
            15,
            "services.api.command = [\"serve\", \"--port\", \"8080\",",
        )
        + &compose_line(19, "services.api.ports = []");
    assert_eq!(stdout(&output), expected);
    assert_eq!(output.status.code(), Some(0_i32));
}

// ── 2. JSON 出力の完全一致 ──────────────────────────────────────────────────

#[test]
fn a_json_run_prints_eight_keys_in_a_fixed_order() {
    let output = spawn(&["--json", "cancelled()", FIXTURE]).expect("バイナリを起動できるはず");
    let expected = "{\"file\":\"".to_owned()
        + FIXTURE
        + "\",\"line\":33,\"column\":18,\"pointer\":\"/jobs/frontend-check/steps/3/if\","
        + "\"path\":\"jobs.frontend-check.steps[3] \\\"Audit (fail on high/critical)\\\" .if\","
        + "\"label\":\"Audit (fail on high/critical)\",\"value\":\"${{ !cancelled() }}\","
        + "\"kind\":\"value\"}\n"
        + "{\"file\":\""
        + FIXTURE
        + "\",\"line\":46,\"column\":18,\"pointer\":\"/jobs/e2e/steps/2/if\","
        + "\"path\":\"jobs.e2e.steps[2] \\\"Upload Playwright report\\\" .if\","
        + "\"label\":\"Upload Playwright report\",\"value\":\"${{ !cancelled() }}\","
        + "\"kind\":\"value\"}\n";
    assert_eq!(stdout(&output), expected);
    assert_eq!(output.status.code(), Some(0_i32));
}

/// `--json --comments` の期待出力（5行）。**行の並びも含めて完全一致で固定する**。
///
/// ルートに書かれたコメントは `pointer` も `path` も空文字列である
/// （RFC 6901 で空の Pointer が文書全体を指す）。
fn expected_json_with_comments() -> String {
    let mut text = String::new();
    for tail in [
        "\",\"line\":4,\"column\":20,\"pointer\":\"\",\"path\":\"\",\"label\":null,\
         \"value\":\"#    候補パーサは、下の3つの `cancelled()` を **別物として区別できなければならない**。\",\
         \"kind\":\"comment\"}",
        "\",\"line\":29,\"column\":23,\"pointer\":\"/jobs/frontend-check/steps\",\
         \"path\":\"jobs.frontend-check.steps\",\"label\":null,\
         \"value\":\"# 1) 散文。ここに書かれた cancelled() は設定値ではない。\",\"kind\":\"comment\"}",
        "\",\"line\":30,\"column\":13,\"pointer\":\"/jobs/frontend-check/steps\",\
         \"path\":\"jobs.frontend-check.steps\",\"label\":null,\
         \"value\":\"#    !cancelled() を使う理由を説明しているだけで、実行条件ではない。\",\"kind\":\"comment\"}",
        "\",\"line\":33,\"column\":18,\"pointer\":\"/jobs/frontend-check/steps/3/if\",\
         \"path\":\"jobs.frontend-check.steps[3] \\\"Audit (fail on high/critical)\\\" .if\",\
         \"label\":\"Audit (fail on high/critical)\",\"value\":\"${{ !cancelled() }}\",\"kind\":\"value\"}",
        "\",\"line\":46,\"column\":18,\"pointer\":\"/jobs/e2e/steps/2/if\",\
         \"path\":\"jobs.e2e.steps[2] \\\"Upload Playwright report\\\" .if\",\
         \"label\":\"Upload Playwright report\",\"value\":\"${{ !cancelled() }}\",\"kind\":\"value\"}",
    ] {
        text.push_str("{\"file\":\"");
        text.push_str(FIXTURE);
        text.push_str(tail);
        text.push('\n');
    }
    text
}

/// 🔑 `kind` は旗の有無によらず 8 番目に必ず出る。キーの数が入力で変わると、
/// 受け手が「今回は出ていないだけ」と「そういう値だった」を区別できない。
#[test]
fn a_json_run_with_comments_marks_every_line() {
    let output =
        spawn(&["--json", "--comments", "cancelled()", FIXTURE]).expect("バイナリを起動できるはず");
    assert_eq!(stdout(&output), expected_json_with_comments());
    assert_eq!(stderr(&output), "");
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
        "scopegrep: ".to_owned() + ANCHOR + ":5: アンカー（&name）は読めない構文である\n";
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
    for arguments in [vec![], vec!["--"], vec!["--nope", "x"]] {
        let output = spawn(&arguments).expect("バイナリを起動できるはず");
        assert_eq!(
            stderr(&output),
            "scopegrep: usage: scopegrep [-i] [--json] [--comments] \
             [--scope <pattern>] <needle> [<path>...]\n"
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

/// `.yml` / `.yaml` は読む・`.txt` は読まない・依存ディレクトリは掘らない・並びはバイト順。
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

// ── 8. 依存ディレクトリを走査から外す ───────────────────────────────────────

/// 🔴 名指しされたパスは除外しない。**除外は「何があるか知らない場所を掘るとき」の話**である。
#[test]
fn a_named_dependency_directory_is_still_read() {
    let root = temporary("named-dependency");
    seed_tree(&root).expect("一時ディレクトリを作れるはず");
    let inside = root.join("node_modules").display().to_string();
    let file = root.join("vendor/hidden.yml").display().to_string();

    let walked = spawn(&["target", &inside]).expect("バイナリを起動できるはず");
    let named = spawn(&["target", &file]).expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("一時ディレクトリを片付けられるはず");

    assert_eq!(
        stdout(&walked),
        seeded_line(&root, "node_modules/hidden.yml")
    );
    assert_eq!(walked.status.code(), Some(0_i32));
    assert_eq!(stdout(&named), seeded_line(&root, "vendor/hidden.yml"));
    assert_eq!(named.status.code(), Some(0_i32));
}

// ── 9. パスを省略したら「今いる場所」───────────────────────────────────────

/// 🔴 省略したときは `./` を付けない。**明示的に `.` を渡したときは付ける**
/// （`grep -rn x .` と同じ。与えたパスをそのまま使う規則を崩さない）。
#[test]
fn an_omitted_path_walks_the_current_directory_without_a_prefix() {
    let root = temporary("cwd");
    seed_tree(&root).expect("一時ディレクトリを作れるはず");

    let omitted = spawn_in(&root, &["target"]).expect("バイナリを起動できるはず");
    let explicit = spawn_in(&root, &["target", "."]).expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("一時ディレクトリを片付けられるはず");

    assert_eq!(
        stdout(&omitted),
        "a.yaml:3: steps[0] \"A\" .run = target\n\
         m.yml:3: steps[0] \"A\" .run = target\n\
         z/inner.yml:3: steps[0] \"A\" .run = target\n"
    );
    assert_eq!(stderr(&omitted), "");
    assert_eq!(omitted.status.code(), Some(0_i32));
    assert_eq!(
        stdout(&explicit),
        "./a.yaml:3: steps[0] \"A\" .run = target\n\
         ./m.yml:3: steps[0] \"A\" .run = target\n\
         ./z/inner.yml:3: steps[0] \"A\" .run = target\n"
    );
}

// ── 10. `--scope` — 構造で絞る ──────────────────────────────────────────────

/// 🔴 これが「構造で絞る」ということ。needle が空でも、**所属が合う値だけ**が並ぶ。
#[test]
fn a_scope_pattern_lists_every_value_at_that_place() {
    let output =
        spawn(&["--scope", "/jobs/*/steps/*/if", "", FIXTURE]).expect("バイナリを起動できるはず");
    let expected = FIXTURE.to_owned()
        + ":33: jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if = ${{ !cancelled() }}\n"
        + FIXTURE
        + ":46: jobs.e2e.steps[2] \"Upload Playwright report\" .if = ${{ !cancelled() }}\n";
    assert_eq!(stdout(&output), expected);
    assert_eq!(stderr(&output), "");
    assert_eq!(output.status.code(), Some(0_i32));
}

/// `**` はどの深さにも当たる。`*` は**ちょうど1つ**なので、同じ場所には当たらない。
#[test]
fn a_double_star_matches_at_any_depth() {
    let deep = spawn(&["--scope", "/**/uses", "", FIXTURE]).expect("バイナリを起動できるはず");
    let shallow = spawn(&["--scope", "/*/uses", "", FIXTURE]).expect("バイナリを起動できるはず");
    let expected = FIXTURE.to_owned()
        + ":21: jobs.frontend-check.steps[0].uses = actions/checkout@v4\n"
        + FIXTURE
        + ":39: jobs.e2e.steps[0].uses = actions/checkout@v4\n"
        + FIXTURE
        + ":47: jobs.e2e.steps[2] \"Upload Playwright report\" .uses = actions/upload-artifact@v4\n";
    assert_eq!(stdout(&deep), expected);
    assert_eq!(deep.status.code(), Some(0_i32));
    assert_eq!(stdout(&shallow), "");
    assert_eq!(shallow.status.code(), Some(1_i32));
}

/// 🔴 パターンが読めなければ**理由を言って** 2 で終わる。黙って全件返さない。
#[test]
fn a_broken_scope_pattern_is_a_usage_error() {
    for pattern in ["jobs/steps", "", "/jobs//steps"] {
        let output = spawn(&["--scope", pattern, "x", FIXTURE]).expect("バイナリを起動できるはず");
        assert_eq!(stdout(&output), "");
        assert!(
            stderr(&output).starts_with("scopegrep: --scope: "),
            "実際の出力: {}",
            stderr(&output)
        );
        assert_eq!(output.status.code(), Some(2_i32));
    }
}

/// 🔑 2回書いたら後勝ちにしない。**どちらが効いたか分からない状態を作らない。**
#[test]
fn a_repeated_scope_flag_is_a_usage_error() {
    let output =
        spawn(&["--scope", "/a", "--scope", "/b", "x", FIXTURE]).expect("バイナリを起動できるはず");
    assert_eq!(stdout(&output), "");
    assert_eq!(output.status.code(), Some(2_i32));
}

/// 値を伴わない `--scope` も使い方の誤りである。
#[test]
fn a_scope_flag_without_a_pattern_is_a_usage_error() {
    let output = spawn(&["x", FIXTURE, "--scope"]).expect("バイナリを起動できるはず");
    assert_eq!(stdout(&output), "");
    assert_eq!(output.status.code(), Some(2_i32));
}

// ── 11. `-i` — 大文字小文字を無視する ───────────────────────────────────────

/// 🔑 無視するのは**照合だけ**である。値は原文のまま返る。
#[test]
fn ignore_case_matches_regardless_of_case() {
    let folded = spawn(&["-i", "CANCELLED()", FIXTURE]).expect("バイナリを起動できるはず");
    let exact = spawn(&["CANCELLED()", FIXTURE]).expect("バイナリを起動できるはず");
    let expected = FIXTURE.to_owned()
        + ":33: jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if = ${{ !cancelled() }}\n"
        + FIXTURE
        + ":46: jobs.e2e.steps[2] \"Upload Playwright report\" .if = ${{ !cancelled() }}\n";
    assert_eq!(stdout(&folded), expected);
    assert_eq!(folded.status.code(), Some(0_i32));
    assert_eq!(stdout(&exact), "", "既定では大文字小文字を区別する");
    assert_eq!(exact.status.code(), Some(1_i32));
}

/// 🔴 列は**原文の一致位置**である。`--ignore-case` は `-i` と同じ意味。
#[test]
fn ignore_case_reports_the_column_of_the_original_text() {
    let root = temporary("fold");
    fs::create_dir_all(&root).expect("一時ディレクトリを作れるはず");
    fs::write(root.join("a.yml"), "note: STRAßE İstanbul Ziel\n").expect("fixture を書けるはず");

    let output =
        spawn_in(&root, &["--json", "--ignore-case", "ziel"]).expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("一時ディレクトリを片付けられるはず");

    // 🔑 `İ` の小文字は2文字（`i` ＋ 合成用の点）である。小文字化した文字列の上で
    //    位置を数える実装なら、ここが 24 にずれる。数えるのは**原文**の文字数である。
    assert_eq!(
        stdout(&output),
        "{\"file\":\"a.yml\",\"line\":1,\"column\":23,\"pointer\":\"/note\",\"path\":\"note\",\
         \"label\":null,\"value\":\"STRAßE İstanbul Ziel\",\"kind\":\"value\"}\n"
    );
    assert_eq!(output.status.code(), Some(0_i32));
}
