//! scopegrep — ヒットした値が、構造のどこに属するかを返す。
//!
//! `grep` は「その行がある」ことしか返さない。YAML の入れ子はテキストの行番号に
//! 現れないので、`.github/workflows/*.yml` を検索しても
//! **その条件がどのステップに付いているか** は分からない。
//!
//! ```console
//! $ scopegrep 'cancelled()' .github/workflows/
//! .github/workflows/ci.yml:33: jobs.frontend-check.steps[3] "Audit" .if = ${{ !cancelled() }}
//! ```
//!
//! 設計は `docs/design/scopegrep.md`。
//!
//! # このファイルの役割
//!
//! **配線点である**（RS-015）。プロセス環境（引数）を読むのはここだけで、
//! 以降のモジュールは値として受け取る。`scopegrep-core` は `no_std` なので、
//! 環境に触ろうとしても**名前解決エラーになる**。

mod argument;
mod cli;
mod invocation;
mod options;
mod outcome;
mod output;
mod output_format;
mod render;
mod run;
mod usage_error;
mod walk;

use std::env;
use std::process::ExitCode;

use crate::invocation::Invocation;
use crate::outcome::Outcome;

// 🔴 終了コードは `main` から**返す**。`std::process::exit` は forbid である
//    （RS-005）。返す形なら後始末が走り、テストからも同じ経路で確かめられる。
fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1_usize).collect();
    match cli::parse(&arguments) {
        Ok(Invocation::Search(options)) => run::search(&options).code(),
        Ok(Invocation::Help) => {
            output::help();
            ExitCode::SUCCESS
        }
        Ok(Invocation::Version) => {
            output::version();
            ExitCode::SUCCESS
        }
        Err(error) => {
            output::usage(error);
            Outcome::Failed.code()
        }
    }
}
