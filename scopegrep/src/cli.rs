//! 引数の解析。**手書きである**（依存 0・ARC-004）。
//!
//! `clap` を使うと引数の形を宣言で書けるが、依存を1つ増やす ADR になる。
//! この道具の引数は `[--json] <needle> <path>...` だけなので、まだ要らない。

use std::path::PathBuf;

use crate::argument::Argument;
use crate::invocation::Invocation;
use crate::options::Options;
use crate::output_format::OutputFormat;
use crate::usage_error::UsageError;

/// 引数列を読む。
///
/// `--` より後は旗として解釈しない。`--json` は `--` より前ならどこに書いてもよい。
///
/// # Errors
///
/// 知らない旗・needle が無い・パスが1つも無い場合は [`UsageError`] を返す。
pub(crate) fn parse(arguments: &[String]) -> Result<Invocation, UsageError> {
    let (head, tail) = split_at_separator(arguments);
    let mut format = OutputFormat::Human;
    let mut positional: Vec<&str> = Vec::new();

    for argument in head {
        match Argument::read(argument) {
            Argument::Json => format = OutputFormat::Json,
            Argument::Help => return Ok(Invocation::Help),
            Argument::Version => return Ok(Invocation::Version),
            Argument::Positional(text) => positional.push(text),
            Argument::Unknown => return Err(UsageError),
        }
    }
    positional.extend(tail.iter().map(String::as_str));

    let (needle, paths) = positional.split_first().ok_or(UsageError)?;
    if paths.is_empty() {
        return Err(UsageError);
    }
    let places = paths.iter().map(PathBuf::from).collect();
    Ok(Invocation::Search(Options::new(
        (*needle).to_owned(),
        places,
        format,
    )))
}

/// 最初の `--` で前後に割る。`--` が無ければ全てが前半である。
fn split_at_separator(arguments: &[String]) -> (&[String], &[String]) {
    let Some(at) = arguments.iter().position(|argument| argument == "--") else {
        return (arguments, &[]);
    };
    let head = arguments.get(..at).unwrap_or(&[]);
    let tail = arguments.get(at.saturating_add(1_usize)..).unwrap_or(&[]);
    (head, tail)
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::invocation::Invocation;
    use crate::options::Options;
    use crate::output_format::OutputFormat;
    use std::path::PathBuf;

    fn words(arguments: &[&str]) -> Vec<String> {
        arguments.iter().map(|word| (*word).to_owned()).collect()
    }

    fn options(arguments: &[&str]) -> Options {
        match parse(&words(arguments)).expect("読めるはず") {
            Invocation::Search(found) => found,
            Invocation::Help | Invocation::Version => panic!("検索のはず"),
        }
    }

    fn paths(arguments: &[&str]) -> Vec<PathBuf> {
        options(arguments).paths().to_vec()
    }

    #[test]
    fn the_first_positional_is_the_needle() {
        let found = options(&["cancelled()", "a.yml", "b.yml"]);
        assert_eq!(found.needle(), "cancelled()");
        assert_eq!(
            found.paths(),
            [PathBuf::from("a.yml"), PathBuf::from("b.yml")]
        );
        assert_eq!(found.format(), OutputFormat::Human);
    }

    #[test]
    fn json_may_come_before_the_needle() {
        assert_eq!(
            options(&["--json", "x", "a.yml"]).format(),
            OutputFormat::Json
        );
    }

    /// 旗は `--` より前ならどこに書いてもよい。位置で意味が変わらない。
    #[test]
    fn json_may_come_after_the_paths() {
        let found = options(&["x", "a.yml", "--json"]);
        assert_eq!(found.format(), OutputFormat::Json);
        assert_eq!(found.needle(), "x");
        assert_eq!(found.paths(), [PathBuf::from("a.yml")]);
    }

    /// `--` から後ろは旗として読まない。**needle が `-` で始まる唯一の逃げ道**である。
    #[test]
    fn everything_after_the_separator_is_positional() {
        let found = options(&["--json", "--", "--json", "-x.yml"]);
        assert_eq!(found.format(), OutputFormat::Json);
        assert_eq!(found.needle(), "--json");
        assert_eq!(found.paths(), [PathBuf::from("-x.yml")]);
    }

    #[test]
    fn a_lone_dash_is_a_path_not_a_flag() {
        assert_eq!(paths(&["x", "-"]), [PathBuf::from("-")]);
    }

    #[test]
    fn help_and_version_win_over_the_rest() {
        assert_eq!(parse(&words(&["-h"])), Ok(Invocation::Help));
        assert_eq!(
            parse(&words(&["--json", "--help", "x"])),
            Ok(Invocation::Help)
        );
        assert_eq!(parse(&words(&["-V"])), Ok(Invocation::Version));
        assert_eq!(parse(&words(&["--version"])), Ok(Invocation::Version));
    }

    #[test]
    fn a_needle_without_a_path_is_a_usage_error() {
        assert!(parse(&words(&[])).is_err());
        assert!(parse(&words(&["cancelled()"])).is_err());
        assert!(parse(&words(&["cancelled()", "--"])).is_err());
    }

    #[test]
    fn an_unknown_flag_is_a_usage_error() {
        assert!(parse(&words(&["--show-scope", "x", "a.yml"])).is_err());
        assert!(parse(&words(&["-n", "x", "a.yml"])).is_err());
    }
}
