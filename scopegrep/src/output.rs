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
    <needle>   the fixed string to search for (not a regular expression;
               case-sensitive by default)
    <path>     a file is read whatever its extension.
               a directory is walked, reading only .yml / .yaml.
               omitted, it walks where you are (no ./ in the output)

options:
    -i, --ignore-case   match case-insensitively (the column stays that of the
                        original text)
    -e, --regex <re>    search with a regular expression instead of a fixed
                        string. exclusive with <needle>; given it, every
                        positional argument is a <path>.
                        a match is within one line (^ and $ are the ends of the
                        line).
                        only a binary built with --features regex has it
    --scope <pattern>   narrow by where a value belongs. a JSON Pointer shape,
                        where * is one segment and ** is zero or more
                        (example: /jobs/*/steps/*/if)
    --json              print one hit per line as JSON Lines
    --comments          return matches inside comments too, marked as comments
    --                  do not read anything after this as a flag
    -h, --help          print this usage
    -V, --version       print the version

exit status:
    0   at least one hit
    1   no hit
    2   an error occurred (2 even when there were hits)";

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
        "scopegrep — returns where in the structure a matched value belongs\n\n\
         usage:\n    {USAGE}\n    scopegrep --help | --version\n\n\
         {HELP_DETAIL}\n\n\
         skipped directories (only when walking; a named path is read):\n    {}",
        walk::SKIPPED_DIRECTORIES.join(" ")
    ));
}

/// この binary が正規表現を持っているか。**構成が版と同じ重さの事実である**（ADR 0002）。
///
/// 🔴 同じ版で振る舞いが違う binary が2つ在るので、`--version` に出す。
/// 出さないと、`-e` が失敗した人が「壊れている」と読む。
#[cfg(feature = "regex")]
const REGEX_STATE: &str = "on";
#[cfg(not(feature = "regex"))]
const REGEX_STATE: &str = "off";

/// 版を標準出力へ出す。版の正本は `Cargo.toml` である。
pub(crate) fn version() {
    to_stdout(&format!(
        "scopegrep {} (regex: {REGEX_STATE})",
        env!("CARGO_PKG_VERSION")
    ));
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
pub(crate) fn usage(error: &UsageError) {
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
