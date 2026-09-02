//! バイナリを実際に起動して、出力と終了コードを固定する統合テスト（QLT-007）。
//!
//! 🔴 **出力の完全一致を見る。** 設計メモが約束している表の形が、実際に出ている形で
//! あることを機械で確かめるのがここの仕事である。この道具は `git` と `gh` を
//! 起動するのが仕事なので、**その 2 つを本物と同じ経路で呼ばせる**——
//! `git` は本物を、`gh` は固定の JSON を返す偽物を `PATH` に置いて呼ばせる。
//!
//! 🔑 clippy がテストで `unwrap` / `expect` を許すのは **`#[test]` 関数の中だけ**である。
//! だから補助関数は落ちない形で書き、落ちる判断は必ずテスト本体に置く。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// 1 日の秒数。
const SECONDS_PER_DAY: u64 = 86_400;

/// 一意な一時ディレクトリのパス。**テストごとに違う名前**にする（並列に走るため）。
fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("fleet-top-cli-{}-{name}", std::process::id()))
}

/// 走査するディレクトリと、そこだけを見る git の設定ファイルを作る。
///
/// 🔑 **手元の `~/.gitconfig` を見せない。** `status.showUntrackedFiles` の類を
/// 設定している環境で表が変わると、テストが人によって落ちる。
fn make_root(name: &str) -> io::Result<PathBuf> {
    let root = temporary(name);
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(&root)?;
    fs::write(root.join("gitconfig"), "")?;
    Ok(root)
}

/// git を起動する。author / committer は引数で渡す（環境に依存させない）。
fn git(root: &Path, arguments: &[&str]) -> io::Result<Output> {
    Command::new("git")
        .args(["-c", "user.email=test@example.invalid"])
        .args(["-c", "user.name=Test"])
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", root.join("gitconfig"))
        .env("GIT_CONFIG_SYSTEM", root.join("gitconfig"))
        .output()
}

/// コミットが 1 つある `main` の空リポジトリを作る。
fn seed_repository(root: &Path, name: &str) -> io::Result<PathBuf> {
    let path = root.join(name);
    let text = path.display().to_string();
    git(root, &["init", "-b", "main", &text])?;
    fs::write(path.join("README.md"), "seed\n")?;
    git(root, &["-C", &text, "add", "README.md"])?;
    git(root, &["-C", &text, "commit", "-m", "seed"])?;
    Ok(path)
}

/// バイナリを起動する。起動できないことの判断はテスト本体に任せる。
fn spawn(root: &Path, arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_fleet-top"))
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", root.join("gitconfig"))
        .env("GIT_CONFIG_SYSTEM", root.join("gitconfig"))
        .output()
}

/// `PATH` を差し替えてバイナリを起動する（`gh` を偽物に、または無くする）。
fn spawn_with_path(root: &Path, bin: &Path, arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_fleet-top"))
        .args(arguments)
        .env("PATH", bin)
        .env("GIT_CONFIG_GLOBAL", root.join("gitconfig"))
        .env("GIT_CONFIG_SYSTEM", root.join("gitconfig"))
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

/// 標準エラーの最後の 1 行（要約）。
fn last_line(output: &Output) -> String {
    stderr(output)
        .lines()
        .next_back()
        .unwrap_or_default()
        .to_owned()
}

// ── 1. `--no-github` の完全一致 ─────────────────────────────────────────────

/// 🔴 表の形（列幅・記号・並び順）を 1 文字も動かさない。
///
/// `alpha` はきれいな `main`、`beta` は detached で変更 1 ＋ 未追跡 1、
/// `not-a-repo` は `.git` を持たないので**行に出ない**。
#[test]
fn a_local_only_run_prints_one_row_per_repository() {
    let root = make_root("no-github").expect("一時ディレクトリを作れるはず");
    seed_repository(&root, "alpha").expect("リポジトリを作れるはず");
    let beta = seed_repository(&root, "beta").expect("リポジトリを作れるはず");
    let text = beta.display().to_string();
    git(&root, &["-C", &text, "checkout", "--detach"]).expect("detach できるはず");
    fs::write(beta.join("README.md"), "changed\n").expect("書けるはず");
    fs::write(beta.join("untracked.txt"), "new\n").expect("書けるはず");
    fs::create_dir_all(root.join("not-a-repo")).expect("作れるはず");

    let output = spawn(&root, &["--no-github", &root.display().to_string()])
        .expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("片付けられるはず");

    assert_eq!(
        stdout(&output),
        "REPO   BRANCH      DIRTY  AHEAD/BEHIND  PR   CI   STALE\n\
         alpha  main        -      (none)        n/a  n/a  n/a\n\
         beta   (detached)  2      (none)        n/a  n/a  n/a\n"
    );
    assert!(
        last_line(&output).starts_with("fleet-top: 2 repos, 0 on GitHub, "),
        "実際の要約: {}",
        last_line(&output)
    );
    assert_eq!(output.status.code(), Some(0_i32));
}

// ── 2. 偽の `gh` で GitHub の経路を通す ─────────────────────────────────────

/// `PATH` に置く `git` と `gh` だけのディレクトリを作る。
///
/// 🔑 **本物の `gh` を見せない。** `/usr/bin` を `PATH` に足すと、
/// 「`gh` が無いとき」の試験が本物の `gh` を叩いてしまう。
#[cfg(unix)]
fn make_bin(root: &Path) -> io::Result<PathBuf> {
    let bin = root.join("bin");
    fs::create_dir_all(&bin)?;
    let found = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect::<Vec<PathBuf>>())
        .unwrap_or_default()
        .into_iter()
        .map(|directory| directory.join("git"))
        .find(|candidate| candidate.is_file());
    match found {
        Some(git_path) => std::os::unix::fs::symlink(git_path, bin.join("git"))?,
        None => return Err(io::Error::other("PATH に git が無い")),
    }
    Ok(bin)
}

/// 引数を無視して固定の JSON を返す `gh` を置く。
#[cfg(unix)]
fn place_fake_gh(bin: &Path, json: &str, code: u8) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let path = bin.join("gh");
    // 🔑 `printf` はシェルの組み込みである。`PATH` に本物の `gh` を見せないために
    //    このディレクトリしか渡していないので、外部コマンド（`cat` 等）は呼べない。
    fs::write(
        &path,
        format!("#!/bin/sh\nprintf '%s\\n' '{json}'\nexit {code}\n"),
    )?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

/// 1970-01-01 からの日数を `YYYY-MM-DD` にする（Howard Hinnant の `civil_from_days`）。
///
/// 🔴 **固定の日付を JSON に書かない。** 書くと明日には「100 日前」が 101 日前になり、
/// いつか境界をまたいで落ちる。日付は毎回**今日から**計算する。
fn iso_date(day: i64) -> String {
    let shifted = day + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day_of_month = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day_of_month:02}")
}

/// 今日（UTC）の 1970-01-01 からの日数。
fn today() -> i64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default();
    i64::try_from(seconds / SECONDS_PER_DAY).unwrap_or_default()
}

/// `alpha` 1 つだけを持ち、origin が GitHub を指す走査対象を作る。
#[cfg(unix)]
fn seed_github_root(name: &str) -> io::Result<(PathBuf, PathBuf)> {
    let root = make_root(name)?;
    let alpha = seed_repository(&root, "alpha")?;
    let text = alpha.display().to_string();
    git(
        &root,
        &[
            "-C",
            &text,
            "remote",
            "add",
            "origin",
            "https://github.com/example-org/alpha.git",
        ],
    )?;
    let bin = make_bin(&root)?;
    Ok((root, bin))
}

/// 🔴 GitHub の 3 列が実際に埋まる。既定枝の CI・open PR・古い枝が 1 本。
#[cfg(unix)]
#[test]
fn a_github_run_fills_the_three_remote_columns() {
    let (root, bin) = seed_github_root("github-ok").expect("一時ディレクトリを作れるはず");
    let fresh = iso_date(today());
    let old = iso_date(today() - 100);
    let json = format!(
        "{{\"data\":{{\"r0\":{{\"nameWithOwner\":\"example-org/alpha\",\
         \"defaultBranchRef\":{{\"name\":\"main\",\"target\":{{\
         \"committedDate\":\"{fresh}T00:00:00Z\",\"statusCheckRollup\":{{\"state\":\"SUCCESS\"}}}}}},\
         \"pullRequests\":{{\"totalCount\":2}},\
         \"refs\":{{\"totalCount\":2,\"nodes\":[\
         {{\"name\":\"main\",\"target\":{{\"committedDate\":\"{fresh}T00:00:00Z\"}}}},\
         {{\"name\":\"old\",\"target\":{{\"committedDate\":\"{old}T00:00:00Z\"}}}}]}}}}}}}}"
    );
    place_fake_gh(&bin, &json, 0_u8).expect("偽の gh を置けるはず");

    let output = spawn_with_path(&root, &bin, &[&root.display().to_string()])
        .expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("片付けられるはず");

    assert_eq!(
        stdout(&output),
        "REPO   BRANCH  DIRTY  AHEAD/BEHIND  PR  CI  STALE\n\
         alpha  main    -      (none)        2   ok  1\n"
    );
    assert!(
        last_line(&output).starts_with("fleet-top: 1 repos, 1 on GitHub, "),
        "実際の要約: {}",
        last_line(&output)
    );
    assert_eq!(output.status.code(), Some(0_i32));
}

/// 🔴 枝が 100 本を超えると STALE は `?` になり、**理由が stderr に出て**終了コードは 1。
///
/// 失敗ではないが「読めなかった」には違いない。理由の無い `?` を出さない（設計メモ F-5）。
#[cfg(unix)]
#[test]
fn too_many_branches_leave_stale_unknown_with_a_reason() {
    let (root, bin) = seed_github_root("github-truncated").expect("一時ディレクトリを作れるはず");
    let fresh = iso_date(today());
    let json = format!(
        "{{\"data\":{{\"r0\":{{\"nameWithOwner\":\"example-org/alpha\",\
         \"defaultBranchRef\":{{\"name\":\"main\",\"target\":{{\
         \"committedDate\":\"{fresh}T00:00:00Z\",\"statusCheckRollup\":null}}}},\
         \"pullRequests\":{{\"totalCount\":0}},\
         \"refs\":{{\"totalCount\":150,\"nodes\":[\
         {{\"name\":\"main\",\"target\":{{\"committedDate\":\"{fresh}T00:00:00Z\"}}}}]}}}}}}}}"
    );
    place_fake_gh(&bin, &json, 0_u8).expect("偽の gh を置けるはず");

    let output = spawn_with_path(&root, &bin, &[&root.display().to_string()])
        .expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("片付けられるはず");

    assert_eq!(
        stdout(&output),
        "REPO   BRANCH  DIRTY  AHEAD/BEHIND  PR  CI  STALE\n\
         alpha  main    -      (none)        -   -   ?\n"
    );
    assert!(
        stderr(&output).contains("fleet-top: alpha: 枝が 100 本を超えている"),
        "実際の stderr: {}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(1_i32));
}

/// 🔴 リクエスト全体が拒まれたら 3 列は `?` で、**行は消えない**。終了コードは 1。
#[cfg(unix)]
#[test]
fn a_rejected_request_leaves_the_row_with_question_marks() {
    let (root, bin) = seed_github_root("github-rejected").expect("一時ディレクトリを作れるはず");
    place_fake_gh(&bin, "{\"message\":\"Bad credentials\"}", 1_u8).expect("偽の gh を置けるはず");

    let output = spawn_with_path(&root, &bin, &[&root.display().to_string()])
        .expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("片付けられるはず");

    assert_eq!(
        stdout(&output),
        "REPO   BRANCH  DIRTY  AHEAD/BEHIND  PR  CI  STALE\n\
         alpha  main    -      (none)        ?   ?   ?\n"
    );
    assert!(
        stderr(&output).contains("fleet-top: alpha: Bad credentials"),
        "実際の標準エラー: {}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(1_i32));
}

/// 🔴 `gh` が入っていない環境でも、**表は出る**。GitHub の 3 列だけが `?` になる。
#[cfg(unix)]
#[test]
fn a_missing_gh_is_reported_without_losing_the_table() {
    let (root, bin) = seed_github_root("github-missing").expect("一時ディレクトリを作れるはず");

    let output = spawn_with_path(&root, &bin, &[&root.display().to_string()])
        .expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("片付けられるはず");

    assert_eq!(
        stdout(&output),
        "REPO   BRANCH  DIRTY  AHEAD/BEHIND  PR  CI  STALE\n\
         alpha  main    -      (none)        ?   ?   ?\n"
    );
    assert!(
        stderr(&output).contains("fleet-top: alpha: "),
        "実際の標準エラー: {}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(1_i32));
}

/// 🔴 `gh` は `errors` があると終了コード 1 を返す。**それで捨てない**——
/// stdout を読み、`errors[].path` が指すリポジトリだけを `?` にする。
#[cfg(unix)]
#[test]
fn a_per_repository_error_is_read_from_stdout_not_the_exit_code() {
    let (root, bin) = seed_github_root("github-not-found").expect("一時ディレクトリを作れるはず");
    place_fake_gh(
        &bin,
        "{\"data\":{\"r0\":null},\"errors\":[{\"type\":\"NOT_FOUND\",\"path\":[\"r0\"],\
         \"message\":\"Could not resolve to a Repository with that name.\"}]}",
        1_u8,
    )
    .expect("偽の gh を置けるはず");

    let output = spawn_with_path(&root, &bin, &[&root.display().to_string()])
        .expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("片付けられるはず");

    assert_eq!(
        stdout(&output),
        "REPO   BRANCH  DIRTY  AHEAD/BEHIND  PR  CI  STALE\n\
         alpha  main    -      (none)        ?   ?   ?\n"
    );
    assert!(
        stderr(&output).contains("fleet-top: alpha: GitHub にそのリポジトリが無い"),
        "実際の標準エラー: {}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(1_i32));
}

// ── 3. 使い方の誤り・版 ─────────────────────────────────────────────────────

#[test]
fn an_unreadable_directory_is_reported_and_exits_two() {
    let root = make_root("missing-dir").expect("一時ディレクトリを作れるはず");
    let absent = root.join("no-such-place");
    let output = spawn(&root, &[&absent.display().to_string()]).expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("片付けられるはず");

    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).starts_with(&format!("fleet-top: {}: ", absent.display())),
        "実際の標準エラー: {}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(2_i32));
}

#[test]
fn bad_arguments_print_the_reason_and_exit_two() {
    let root = make_root("bad-arguments").expect("一時ディレクトリを作れるはず");
    for arguments in [
        vec!["--stale-days", "x"],
        vec!["--stale-days"],
        vec!["--depth", "2"],
        vec!["a", "b"],
    ] {
        let output = spawn(&root, &arguments).expect("バイナリを起動できるはず");
        assert_eq!(stdout(&output), "");
        assert!(
            stderr(&output).starts_with("fleet-top: "),
            "実際の標準エラー: {}",
            stderr(&output)
        );
        assert_eq!(output.status.code(), Some(2_i32));
    }
    fs::remove_dir_all(&root).expect("片付けられるはず");
}

#[test]
fn help_and_version_succeed_on_stdout() {
    let root = make_root("help").expect("一時ディレクトリを作れるはず");
    for flag in ["--help", "-h"] {
        let output = spawn(&root, &[flag]).expect("バイナリを起動できるはず");
        assert!(
            stdout(&output).contains("fleet-top [DIR]"),
            "実際の標準出力: {}",
            stdout(&output)
        );
        assert_eq!(stderr(&output), "");
        assert_eq!(output.status.code(), Some(0_i32));
    }
    for flag in ["--version", "-V"] {
        let output = spawn(&root, &[flag]).expect("バイナリを起動できるはず");
        assert_eq!(
            stdout(&output),
            format!("fleet-top {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(output.status.code(), Some(0_i32));
    }
    fs::remove_dir_all(&root).expect("片付けられるはず");
}

// ── 4. リポジトリが 1 つも無いディレクトリ ──────────────────────────────────

/// 🔑 **見出しだけでも出す。** 空の出力は「1 つも無い」と「壊れている」を
/// 区別できない（設計メモ F-5「黙って空にしない」）。
#[test]
fn an_empty_directory_prints_only_the_headings() {
    let root = make_root("empty").expect("一時ディレクトリを作れるはず");
    let output = spawn(&root, &[&root.display().to_string()]).expect("バイナリを起動できるはず");
    fs::remove_dir_all(&root).expect("片付けられるはず");

    assert_eq!(
        stdout(&output),
        "REPO  BRANCH  DIRTY  AHEAD/BEHIND  PR  CI  STALE\n"
    );
    assert!(
        last_line(&output).starts_with("fleet-top: 0 repos, 0 on GitHub, "),
        "実際の要約: {}",
        last_line(&output)
    );
    assert_eq!(output.status.code(), Some(0_i32));
}
