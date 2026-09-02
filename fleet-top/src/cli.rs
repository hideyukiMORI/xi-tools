//! 引数の解析。**手書きである**（依存 0・ARC-004）。
//!
//! `clap` を使うと引数の形を宣言で書けるが、依存を1つ増やす ADR になる。
//! この道具の引数は `[DIR] [--stale-days N] [--no-github]` だけなので、まだ要らない。

use std::path::PathBuf;
use std::slice::Iter;

use crate::argument::Argument;
use crate::github_access::GithubAccess;
use crate::invocation::Invocation;
use crate::options::Options;
use crate::usage_error::UsageError;

/// 何日で「古い枝」と呼ぶか の既定値。
const DEFAULT_STALE_DAYS: u32 = 30;

/// 走査するディレクトリの既定値。
const DEFAULT_DIRECTORY: &str = ".";

/// 引数列を読む。
///
/// `--` より後は旗として解釈しない。旗は `--` より前ならどこに書いてもよい。
/// **ディレクトリを省略したら「今いる場所」**（`.`）になる。
///
/// # Errors
///
/// 知らない旗・ディレクトリが2つ以上・`--stale-days` の値が無い／読めない／
/// 2回書かれた場合は [`UsageError`] を返す。
pub(crate) fn parse(arguments: &[String]) -> Result<Invocation, UsageError> {
    let (head, tail) = split_at_separator(arguments);
    let mut stale_days: Option<u32> = None;
    let mut github = GithubAccess::Query;
    let mut positional: Vec<&str> = Vec::new();

    let mut rest = head.iter();
    while let Some(argument) = rest.next() {
        match Argument::read(argument) {
            Argument::StaleDays => {
                stale_days = Some(read_stale_days(&mut rest, stale_days.is_some())?);
            }
            Argument::NoGithub => github = GithubAccess::Skip,
            Argument::Help => return Ok(Invocation::Help),
            Argument::Version => return Ok(Invocation::Version),
            Argument::Positional(text) => positional.push(text),
            Argument::Unknown => return Err(UsageError::Arguments),
        }
    }
    positional.extend(tail.iter().map(String::as_str));

    Ok(Invocation::Report(Options::new(
        place(&positional)?,
        stale_days.unwrap_or(DEFAULT_STALE_DAYS),
        github,
    )))
}

/// `--stale-days` の値を読む。
///
/// 🔴 2回目は**後勝ちにしない**。どちらが効いたか分からない起動を作らないためで、
/// 「上書きできる」より「書き間違いに気づける」ほうがこの道具では価値が高い。
fn read_stale_days(rest: &mut Iter<'_, String>, given: bool) -> Result<u32, UsageError> {
    if given {
        return Err(UsageError::RepeatedStaleDays);
    }
    let text = rest.next().ok_or(UsageError::StaleDaysWithoutValue)?;
    // 🔑 `map_err(|_| ...)` は理由を捨てる書き方（`map_err_ignore` が deny）なので、
    //    `ok()` で落としてから、打たれた文字を添えて返す。
    text.parse::<u32>()
        .ok()
        .ok_or_else(|| UsageError::StaleDaysNotANumber(text.clone()))
}

/// 走査する場所。**省略したら「今いる場所」**（`.`）で、2つ以上は誤りである。
///
/// 🔑 この道具は「ディレクトリ直下に並んだリポジトリ」を見る（設計メモ F-4）。
/// 場所を複数受けると、同じ名前のリポジトリが2行出て並び順の意味が消える。
fn place(paths: &[&str]) -> Result<PathBuf, UsageError> {
    match *paths {
        [] => Ok(PathBuf::from(DEFAULT_DIRECTORY)),
        [only] => Ok(PathBuf::from(only)),
        _ => Err(UsageError::Arguments),
    }
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
    use crate::github_access::GithubAccess;
    use crate::invocation::Invocation;
    use crate::options::Options;
    use crate::usage_error::UsageError;
    use std::path::PathBuf;

    fn words(arguments: &[&str]) -> Vec<String> {
        arguments.iter().map(|word| (*word).to_owned()).collect()
    }

    fn options(arguments: &[&str]) -> Options {
        match parse(&words(arguments)).expect("読めるはず") {
            Invocation::Report(found) => found,
            Invocation::Help | Invocation::Version => panic!("表を出すはず"),
        }
    }

    /// 旗を1つも付けない既定の形。
    #[test]
    fn an_omitted_directory_becomes_the_current_place() {
        let found = options(&[]);
        assert_eq!(found.directory(), PathBuf::from("."));
        assert_eq!(found.stale_days(), 30_u32);
        assert_eq!(found.github(), GithubAccess::Query);
    }

    #[test]
    fn the_only_positional_is_the_directory() {
        assert_eq!(
            options(&["/home/x/docker"]).directory(),
            PathBuf::from("/home/x/docker")
        );
    }

    /// 旗は `--` より前ならどこに書いてもよい。位置で意味が変わらない。
    #[test]
    fn flags_may_come_before_or_after_the_directory() {
        for arguments in [
            vec!["--stale-days", "7", "--no-github", "src"],
            vec!["src", "--no-github", "--stale-days", "7"],
        ] {
            let found = options(&arguments);
            assert_eq!(found.directory(), PathBuf::from("src"));
            assert_eq!(found.stale_days(), 7_u32);
            assert_eq!(found.github(), GithubAccess::Skip);
        }
    }

    /// `--` から後ろは旗として読まない。**`-` で始まる名前のディレクトリを渡す唯一の逃げ道**である。
    #[test]
    fn everything_after_the_separator_is_positional() {
        let found = options(&["--no-github", "--", "--stale-days"]);
        assert_eq!(found.directory(), PathBuf::from("--stale-days"));
        assert_eq!(found.github(), GithubAccess::Skip);
    }

    #[test]
    fn help_and_version_win_over_the_rest() {
        assert_eq!(parse(&words(&["-h"])), Ok(Invocation::Help));
        assert_eq!(
            parse(&words(&["--no-github", "--help", "."])),
            Ok(Invocation::Help)
        );
        assert_eq!(parse(&words(&["-V"])), Ok(Invocation::Version));
        assert_eq!(parse(&words(&["--version"])), Ok(Invocation::Version));
    }

    /// 🔴 場所は1つまで。2つ受けると同じ名前の行が2つ出る。
    #[test]
    fn two_directories_are_a_usage_error() {
        assert_eq!(parse(&words(&["a", "b"])), Err(UsageError::Arguments));
    }

    #[test]
    fn an_unknown_flag_is_a_usage_error() {
        assert_eq!(parse(&words(&["--depth", "2"])), Err(UsageError::Arguments));
        assert_eq!(parse(&words(&["-n"])), Err(UsageError::Arguments));
        assert_eq!(
            parse(&words(&["--stale-days=7"])),
            Err(UsageError::Arguments),
            "`--stale-days=N` は受けない"
        );
    }

    #[test]
    fn a_stale_days_without_a_value_is_an_error() {
        assert_eq!(
            parse(&words(&[".", "--stale-days"])),
            Err(UsageError::StaleDaysWithoutValue)
        );
    }

    /// 🔑 値は旗として読まない（`--` の前でも）。読むと `--no-github` が日数になる。
    #[test]
    fn a_value_that_is_not_a_number_is_an_error() {
        assert_eq!(
            parse(&words(&["--stale-days", "x"])),
            Err(UsageError::StaleDaysNotANumber(String::from("x")))
        );
        assert_eq!(
            parse(&words(&["--stale-days", "-1"])),
            Err(UsageError::StaleDaysNotANumber(String::from("-1")))
        );
        assert_eq!(
            parse(&words(&["--stale-days", "--no-github"])),
            Err(UsageError::StaleDaysNotANumber(String::from("--no-github")))
        );
    }

    /// 0 は「今日より前は全部古い」であって、誤りではない。
    #[test]
    fn zero_days_is_a_number() {
        assert_eq!(options(&["--stale-days", "0"]).stale_days(), 0_u32);
    }

    /// 🔴 2回書いたら**後勝ちにしない**。
    #[test]
    fn a_repeated_stale_days_is_an_error_not_an_overwrite() {
        assert_eq!(
            parse(&words(&["--stale-days", "7", "--stale-days", "9"])),
            Err(UsageError::RepeatedStaleDays)
        );
    }

    /// `--no-github` は2回書いても意味が変わらない（値を持たない旗である）。
    #[test]
    fn a_repeated_no_github_stays_the_same() {
        assert_eq!(
            options(&["--no-github", "--no-github"]).github(),
            GithubAccess::Skip
        );
    }
}
