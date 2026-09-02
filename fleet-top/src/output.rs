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
    DIR        直下を見るディレクトリ。その中で .git を持つものがリポジトリになる。
               再帰しない。省略したら今いる場所（.）

options:
    --stale-days N   既定枝以外の枝を「古い」と呼ぶまでの日数（既定 30）
    --no-github      gh を起動しない。GitHub の3列は n/a になる（オフライン用）
    --               以降を旗として解釈しない
    -h, --help       この使い方を出す
    -V, --version    版を出す

columns:
    REPO           ディレクトリ名（バイト順）
    BRANCH         いま居る枝。detached なら (detached)
    DIRTY          変更・未追跡・衝突のエントリ数
    AHEAD/BEHIND   上流との差。上流が無ければ (none)
    PR             open な PR の数
    CI             既定枝の先頭コミットの検査（ok / FAIL / ... / -）
    STALE          --stale-days より古い枝の数

    -    ゼロ・該当なし
    n/a  origin が GitHub でない（聞いていない）
    ?    取れなかった（理由は標準エラーに1行ずつ）

exit status:
    0   全行が確定した
    1   ? を含む行がある（表は出ている）
    2   使い方の誤り・DIR が読めない・時計が読めない";

/// 表を標準出力へ出す。**`render` の結果をそのまま**書く（行末に改行が付いている）。
pub(crate) fn table(text: &str) {
    to_stdout(text);
}

/// 使い方を標準出力へ出す（`--help` は成功である）。
pub(crate) fn help() {
    to_stdout(&format!(
        "fleet-top — 数十の git リポジトリの状態を1画面で返す\n\n\
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
        "時計が読めない（1970年より前を指している）: {error}"
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
