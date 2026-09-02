//! **標準出力・標準エラーに書いてよい唯一のモジュール**（RS-014）。
//!
//! 🔑 下の `#[expect]` が「ここだけが出力する」という宣言そのものである。
//! 出力をやめたら `unfulfilled_lint_expectations` が落とすので、
//! **この宣言が腐ったまま残る経路が無い**。
//!
//! 組み立ては `render` が持つ。ここは書き出すだけで、文字列を作らない。

use std::io;
use std::path::Path;

use scopegrep_core::hit::Hit;
use scopegrep_core::parse_error::ParseError;

use crate::output_format::OutputFormat;
use crate::render;
use crate::usage_error::{USAGE, UsageError};
use crate::walk;

/// 使い方の本文。**usage の1行は [`crate::usage_error::USAGE`] が正である**
/// （使い方の誤りを報告するときと同じ文字列を、2箇所に書かない）。
const HELP_DETAIL: &str = "\
arguments:
    <needle>   探す固定文字列（正規表現ではない・既定では大文字小文字を区別する）
    <path>     ファイルなら拡張子を問わず読む。
               ディレクトリなら再帰して .yml / .yaml だけを読む。
               省略したら今いる場所を再帰する（表示に ./ を付けない）

options:
    -i, --ignore-case   大文字小文字を無視して照合する（列は原文の位置）
    --scope <pattern>   所属で絞る。JSON Pointer の形で、* は1セグメント・
                        ** は0個以上（例: /jobs/*/steps/*/if）
    --json              1ヒット1行の JSON Lines で出す
    --comments          コメント内の一致も、コメントだと明示して返す
    --                  以降を旗として解釈しない
    -h, --help          この使い方を出す
    -V, --version       版を出す

exit status:
    0   1件以上ヒットした
    1   ヒットが無かった
    2   エラーがあった（ヒットがあっても 2）";

/// ヒット1件を標準出力へ出す。
pub(crate) fn hit(file: &Path, found: &Hit, format: OutputFormat) {
    let line = match format {
        OutputFormat::Human => render::human(file, found),
        OutputFormat::Json => render::json(file, found),
    };
    to_stdout(&line);
}

/// 使い方を標準出力へ出す（`--help` は成功である）。
///
/// 🔑 走査から外すディレクトリの一覧は [`walk::SKIPPED_DIRECTORIES`] から作る。
/// 一覧を help に手で書くと、リストを直したときに片方だけ古くなる。
pub(crate) fn help() {
    to_stdout(&format!(
        "scopegrep — ヒットした値が、構造のどこに属するかを返す\n\n\
         usage:\n    {USAGE}\n    scopegrep --help | --version\n\n\
         {HELP_DETAIL}\n\n\
         skipped directories (再帰のときだけ。名指しされたパスは読む):\n    {}",
        walk::SKIPPED_DIRECTORIES.join(" ")
    ));
}

/// 版を標準出力へ出す。版の正本は `Cargo.toml` である。
pub(crate) fn version() {
    to_stdout(&format!("scopegrep {}", env!("CARGO_PKG_VERSION")));
}

/// 読めなかったファイルを標準エラーへ報告する。
pub(crate) fn unreadable(file: &Path, error: &io::Error) {
    to_stderr(&format!("{}: {error}", file.display()));
}

/// 読める部分集合の外にあったファイルを標準エラーへ報告する。
///
/// 🔑 **行番号を必ず言う。**「このファイルは読めない」で終わらせると、
/// 部分集合を広げるべきかどうかを誰も判断できない。
pub(crate) fn unparsable(file: &Path, error: &ParseError) {
    to_stderr(&format!(
        "{}:{}: {}",
        file.display(),
        error.line(),
        error.kind()
    ));
}

/// 引数が読めなかったことを標準エラーへ報告する。
pub(crate) fn usage(error: UsageError) {
    to_stderr(&format!("{error}"));
}

// 🔴 出力を行ってよい唯一の関数（RS-014）。ここを増やさない。
#[expect(
    clippy::print_stdout,
    reason = "RS-014: 出力は1箇所に集約する。標準出力に書くのはこの関数だけである"
)]
fn to_stdout(line: &str) {
    println!("{line}");
}

// 🔴 出力を行ってよい唯一の関数（RS-014）。報告は必ず `scopegrep: ` で始める。
#[expect(
    clippy::print_stderr,
    reason = "RS-014: 出力は1箇所に集約する。標準エラーに書くのはこの関数だけである"
)]
fn to_stderr(message: &str) {
    eprintln!("scopegrep: {message}");
}
