//! fleet-top — 数十の git リポジトリの状態を 1 画面で返す。
//!
//! 枝・未コミット・ahead/behind・open PR・CI・古い枝を、ディレクトリ直下の
//! 全リポジトリについて 1 度に出す。手で回すと `gh api` の直列実行で 90 秒かかり、
//! **90 秒かかるコマンドは打たれない**（設計メモ `docs/design/fleet-top.md`）。
//!
//! ```text
//! REPO   BRANCH      DIRTY  AHEAD/BEHIND  PR   CI    STALE
//! alpha  main        -      -             -    ok    -
//! beta   feat/login  3      +2/-1         1    FAIL  2
//! gamma  (detached)  -      (none)        n/a  n/a   n/a
//! ```
//!
//! 取れなかった値は `?` になり、行は消えない。`?` が 1 つでもあれば終了コード 1
//! である（設計メモ F-5）。GitHub は `gh api graphql` を 3 リポジトリずつ
//! 並列に叩く（[ADR 0003](https://github.com/hideyukiMORI/xi-tools/blob/main/docs/adr/0003-fleet-top-fetches-github-via-chunked-graphql.md)）。
//!
//! # このファイルの役割
//!
//! **配線点である**（RS-015）。プロセス環境（引数・時計・経過時間）を読むのはここだけで、
//! 以降のモジュールは値として受け取る。`fleet-top-core` は `no_std` なので、
//! 環境に触ろうとしても**名前解決エラーになる**。

mod argument;
mod chunk_outcome;
mod cli;
mod github;
mod github_access;
mod invocation;
mod local;
mod local_finding;
mod options;
mod outcome;
mod output;
mod parallel;
mod reason;
mod repository;
mod run;
mod scan;
mod tally;
mod target;
mod usage_error;

use std::env;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, SystemTimeError, UNIX_EPOCH};

use fleet_top_core::day::Day;

use crate::invocation::Invocation;
use crate::options::Options;
use crate::outcome::Outcome;

// 🔴 終了コードは `main` から**返す**。`std::process::exit` は forbid である
//    （RS-005）。返す形なら後始末が走り、テストからも同じ経路で確かめられる。
fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1_usize).collect();
    let started = Instant::now();
    match cli::parse(&arguments) {
        Ok(Invocation::Report(options)) => survey(&options, started),
        Ok(Invocation::Help) => {
            output::help();
            ExitCode::SUCCESS
        }
        Ok(Invocation::Version) => {
            output::version();
            ExitCode::SUCCESS
        }
        Err(error) => {
            output::usage(&error);
            Outcome::Usage.code()
        }
    }
}

/// 走査して表を出す。
///
/// 🔑 **「今日」と経過時間を読むのはここだけである**（RS-015）。以降は
/// [`Day`] の値と [`std::time::Duration`] として渡る。同じ値を渡せば、いつ実行しても
/// 同じ表が出る——それが `fleet-top-core` を `no_std` にしている理由でもある。
fn survey(options: &Options, started: Instant) -> ExitCode {
    let today = match today() {
        Ok(day) => day,
        Err(error) => {
            output::clock(&error);
            return Outcome::Usage.code();
        }
    };
    let tally = match run::report(options, today) {
        Ok(found) => found,
        Err(error) => {
            output::unreadable(options.directory(), &error);
            return Outcome::Usage.code();
        }
    };
    output::summary(&tally, started.elapsed());
    tally.outcome().code()
}

/// 今日（UTC）。時計が 1970 年より前を指していれば読めない。
fn today() -> Result<Day, SystemTimeError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| Day::from_unix_seconds(since.as_secs()))
}
