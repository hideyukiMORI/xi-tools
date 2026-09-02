//! **標準出力・標準エラーに書いてよい唯一のモジュール**（RS-014）。
//!
//! 🔑 下の `#[expect]` が「ここだけが出力する」という宣言そのものである。
//! 出力をやめたら `unfulfilled_lint_expectations` が落とすので、
//! **この宣言が腐ったまま残る経路が無い**。
//!
//! 表の組み立ては `fleet_top_core::table::render` が持つ。ここは書き出すだけである。
//!
//! 🔴 **表は stdout、理由と要約は stderr。** 表だけをパイプに流せる形にしておくと、
//! `fleet-top | grep FAIL` が理由の行に当たらない。

use std::io;
use std::path::Path;
use std::time::{Duration, SystemTimeError};

use crate::tally::Tally;
use crate::usage_error::{USAGE, UsageError};

/// 使い方の本文。**usage の1行は [`crate::usage_error::USAGE`] が正である**
/// （使い方の誤りを報告するときと同じ文字列を、2箇所に書かない）。
const HELP_DETAIL: &str = "\
arguments:
    DIR        directory to scan. direct children that have a .git become
               repositories. not recursive. defaults to the current directory (.)

options:
    --stale-days N   days after which a non-default branch is stale (default 30)
    --no-github      do not run gh. the 3 GitHub columns become n/a (offline)
    --               do not read what follows as flags
    -h, --help       print this usage
    -V, --version    print the version

columns:
    REPO           directory name (byte order)
    BRANCH         current branch. (detached) when detached
    DIRTY          changed, untracked and conflicted entries
    AHEAD/BEHIND   difference from the upstream. (none) if there is no upstream
    PR             number of open PRs
    CI             checks on the default branch head (ok / FAIL / ... / -)
    STALE          number of branches older than --stale-days

    -    zero, or not applicable
    n/a  origin is not GitHub (not asked)
    ?    could not be determined (one reason per line on stderr)

exit status:
    0   every row is complete
    1   some row contains ? (the table is still printed)
    2   usage error, cannot read DIR, or cannot read the system clock";

/// 表を標準出力へ出す。**`render` の結果をそのまま**書く（行末に改行が付いている）。
pub(crate) fn table(text: &str) {
    to_stdout(text);
}

/// 使い方を標準出力へ出す（`--help` は成功である）。
pub(crate) fn help() {
    to_stdout(&format!(
        "fleet-top — the state of dozens of git repositories on one screen\n\n\
         usage:\n    {USAGE}\n    fleet-top --help | --version\n\n\
         {HELP_DETAIL}\n"
    ));
}

/// 版を標準出力へ出す。版の正本は `Cargo.toml` である。
pub(crate) fn version() {
    to_stdout(&format!("fleet-top {}\n", env!("CARGO_PKG_VERSION")));
}

/// 引数が読めなかったことを標準エラーへ報告する。
pub(crate) fn usage(error: &UsageError) {
    to_stderr(&format!("{error}"));
}

/// 走査するディレクトリが読めなかったことを標準エラーへ報告する。
pub(crate) fn unreadable(directory: &Path, error: &io::Error) {
    to_stderr(&format!("{}: {error}", directory.display()));
}

/// 時計が読めなかったことを標準エラーへ報告する。
///
/// 🔑 「今日」が無ければ**古い枝を数えられない**。数えられないまま `-` を出すと
/// 「古い枝が無い」に見えるので、表を出さずに 2 で終わる（設計メモ F-5）。
pub(crate) fn clock(error: &SystemTimeError) {
    to_stderr(&format!(
        "cannot read the system clock (it points before 1970): {error}"
    ));
}

/// `?` になったリポジトリの理由を標準エラーへ 1 行で報告する。
///
/// 🔑 **同じ理由で落ちた塊は 1 行にまとめる。** `gh` が入っていない日に、
/// 同じ文が 20 行並ぶのは報告ではなく雑音である。
pub(crate) fn problem(names: &[&str], why: &str) {
    if names.is_empty() {
        return;
    }
    to_stderr(&format!("{}: {why}", names.join(", ")));
}

/// 要約を標準エラーへ出す（**最後の 1 行**）。
pub(crate) fn summary(tally: &Tally, elapsed: Duration) {
    to_stderr(&format!(
        "{} repos, {} on GitHub, {:.1}s",
        tally.repositories(),
        tally.on_github(),
        elapsed.as_secs_f64()
    ));
}

// 🔴 出力を行ってよい唯一の関数（RS-014）。ここを増やさない。
//    改行は呼び手が持つ（`render` の結果は行末に改行を持っている）。
#[expect(
    clippy::print_stdout,
    reason = "RS-014: 出力は1箇所に集約する。標準出力に書くのはこの関数だけである"
)]
fn to_stdout(text: &str) {
    print!("{text}");
}

// 🔴 出力を行ってよい唯一の関数（RS-014）。報告は必ず `fleet-top: ` で始める。
#[expect(
    clippy::print_stderr,
    reason = "RS-014: 出力は1箇所に集約する。標準エラーに書くのはこの関数だけである"
)]
fn to_stderr(message: &str) {
    eprintln!("fleet-top: {message}");
}
