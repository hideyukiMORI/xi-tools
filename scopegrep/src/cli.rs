//! 引数の解析。**手書きである**（依存 0・ARC-004）。
//!
//! `clap` を使うと引数の形を宣言で書けるが、依存を1つ増やす ADR になる。
//! この道具の引数は `[-i] [--json] [--comments] [--scope <pattern>] <needle> [<path>...]`
//! だけなので、まだ要らない。

use std::path::PathBuf;
use std::slice::Iter;

use scopegrep_core::case_match::CaseMatch;
use scopegrep_core::query::Query;
use scopegrep_core::scope_pattern::ScopePattern;
use scopegrep_core::search_scope::SearchScope;

use crate::argument::Argument;
use crate::invocation::Invocation;
use crate::options::Options;
use crate::output_format::OutputFormat;
use crate::usage_error::UsageError;

/// 引数列を読む。
///
/// `--` より後は旗として解釈しない。旗は `--` より前ならどこに書いてもよい。
/// **パスを省略したら「今いる場所」**（空のパス1つ）になる。
///
/// # Errors
///
/// 知らない旗・needle が無い・`--scope` のパターンが読めない場合は
/// [`UsageError`] を返す。
pub(crate) fn parse(arguments: &[String]) -> Result<Invocation, UsageError> {
    let (head, tail) = split_at_separator(arguments);
    let mut format = OutputFormat::Human;
    let mut kinds = SearchScope::Values;
    let mut case = CaseMatch::Exact;
    let mut within: Option<ScopePattern> = None;
    let mut positional: Vec<&str> = Vec::new();

    let mut rest = head.iter();
    while let Some(argument) = rest.next() {
        match Argument::read(argument) {
            Argument::Json => format = OutputFormat::Json,
            Argument::Comments => kinds = SearchScope::ValuesAndComments,
            Argument::IgnoreCase => case = CaseMatch::Fold,
            Argument::Scope => within = Some(read_pattern(&mut rest, within.is_some())?),
            Argument::Help => return Ok(Invocation::Help),
            Argument::Version => return Ok(Invocation::Version),
            Argument::Positional(text) => positional.push(text),
            Argument::Unknown => return Err(UsageError::Arguments),
        }
    }
    positional.extend(tail.iter().map(String::as_str));

    let (needle, paths) = positional.split_first().ok_or(UsageError::Arguments)?;
    Ok(Invocation::Search(Options::new(
        build(needle, case, kinds, within),
        places(paths),
        format,
    )))
}

/// `--scope` の値を読む。
///
/// 🔴 2回目は**後勝ちにしない**。どちらが効いたか分からない起動を作らないためで、
/// 「上書きできる」より「書き間違いに気づける」ほうがこの道具では価値が高い。
fn read_pattern(rest: &mut Iter<'_, String>, given: bool) -> Result<ScopePattern, UsageError> {
    if given {
        return Err(UsageError::RepeatedScope);
    }
    let text = rest.next().ok_or(UsageError::ScopeWithoutPattern)?;
    ScopePattern::parse(text).map_err(UsageError::Scope)
}

/// 旗から検索条件を組む。**既定から広げる方向にしか動かない**。
fn build(needle: &str, case: CaseMatch, kinds: SearchScope, within: Option<ScopePattern>) -> Query {
    let mut query = Query::new(needle);
    match case {
        CaseMatch::Exact => {}
        CaseMatch::Fold => query = query.ignoring_case(),
    }
    match kinds {
        SearchScope::Values => {}
        SearchScope::ValuesAndComments => query = query.including_comments(),
    }
    if let Some(pattern) = within {
        query = query.within(pattern);
    }
    query
}

/// 探す場所。**省略されたら「今いる場所」**（空のパス）1つになる。
///
/// 🔑 `.` を明示的に渡したときと区別する。`grep -rn x` と `grep -rn x .` で
/// 表示が変わるのと同じで、**与えたパスをそのまま使う**規則を崩さないため。
fn places(paths: &[&str]) -> Vec<PathBuf> {
    if paths.is_empty() {
        return vec![PathBuf::new()];
    }
    paths.iter().map(PathBuf::from).collect()
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
    use crate::usage_error::UsageError;
    use scopegrep_core::query::Query;
    use scopegrep_core::scope_pattern::ScopePattern;
    use scopegrep_core::scope_pattern_error::ScopePatternError;
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

    /// 検索条件を、組み上がった [`Query`] そのものと比べる。
    ///
    /// 🔑 `Query` は中身を公開しない（フィールドも取り出し口も非公開）ので、
    /// **同じ物を組んで比べる**のが外から確かめる唯一の形である。
    fn query(arguments: &[&str]) -> Query {
        options(arguments).query().clone()
    }

    #[test]
    fn the_first_positional_is_the_needle() {
        let found = options(&["cancelled()", "a.yml", "b.yml"]);
        assert_eq!(found.query(), &Query::new("cancelled()"));
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

    /// 既定はコメントを探さない。**旗を付けたときだけ**範囲が広がる。
    #[test]
    fn comments_are_off_unless_the_flag_is_given() {
        assert_eq!(
            query(&["x", "a.yml"]),
            Query::new("x"),
            "既定でコメントを探している"
        );
        let found = options(&["x", "a.yml", "--comments"]);
        assert_eq!(found.query(), &Query::new("x").including_comments());
        assert_eq!(found.format(), OutputFormat::Human);
    }

    /// 2つの旗は独立していて、順序でも意味が変わらない。
    #[test]
    fn json_and_comments_are_independent() {
        let found = options(&["--comments", "--json", "x", "a.yml"]);
        assert_eq!(found.format(), OutputFormat::Json);
        assert_eq!(found.query(), &Query::new("x").including_comments());
    }

    /// `--` より後の `--comments` は needle かパスである。
    #[test]
    fn comments_after_the_separator_is_positional() {
        let found = options(&["x", "--", "--comments"]);
        assert_eq!(found.query(), &Query::new("x"));
        assert_eq!(found.paths(), [PathBuf::from("--comments")]);
    }

    /// 旗は `--` より前ならどこに書いてもよい。位置で意味が変わらない。
    #[test]
    fn json_may_come_after_the_paths() {
        let found = options(&["x", "a.yml", "--json"]);
        assert_eq!(found.format(), OutputFormat::Json);
        assert_eq!(found.query(), &Query::new("x"));
        assert_eq!(found.paths(), [PathBuf::from("a.yml")]);
    }

    /// `--` から後ろは旗として読まない。**needle が `-` で始まる唯一の逃げ道**である。
    #[test]
    fn everything_after_the_separator_is_positional() {
        let found = options(&["--json", "--", "--json", "-x.yml"]);
        assert_eq!(found.format(), OutputFormat::Json);
        assert_eq!(found.query(), &Query::new("--json"));
        assert_eq!(found.paths(), [PathBuf::from("-x.yml")]);
    }

    #[test]
    fn a_lone_dash_is_a_path_not_a_flag() {
        assert_eq!(paths(&["x", "-"]), [PathBuf::from("-")]);
    }

    // ── `-i`（大文字小文字）────────────────────────────────────────────────

    /// `-i` と `--ignore-case` は同じ意味。既定は区別する。
    #[test]
    fn ignore_case_has_two_spellings_and_one_meaning() {
        assert_eq!(query(&["x", "a.yml"]), Query::new("x"));
        assert_eq!(
            query(&["-i", "x", "a.yml"]),
            Query::new("x").ignoring_case()
        );
        assert_eq!(
            query(&["--ignore-case", "x", "a.yml"]),
            Query::new("x").ignoring_case()
        );
    }

    // ── `--scope`（所属で絞る）────────────────────────────────────────────

    #[test]
    fn scope_takes_the_next_argument_as_its_pattern() {
        let Ok(pattern) = ScopePattern::parse("/jobs/*/steps/*/if") else {
            panic!("読めるはず");
        };
        let found = options(&["--scope", "/jobs/*/steps/*/if", "x", "a.yml"]);
        assert_eq!(found.query(), &Query::new("x").within(pattern));
        assert_eq!(found.paths(), [PathBuf::from("a.yml")]);
    }

    /// パターンの誤りは**理由付き**で返る。「usage:」だけでは直しようがない。
    #[test]
    fn a_broken_pattern_says_why() {
        assert_eq!(
            parse(&words(&["--scope", "jobs", "x", "a.yml"])),
            Err(UsageError::Scope(ScopePatternError::NotRooted))
        );
        assert_eq!(
            parse(&words(&["--scope", "", "x", "a.yml"])),
            Err(UsageError::Scope(ScopePatternError::Empty))
        );
        assert_eq!(
            parse(&words(&["--scope", "/a//b", "x", "a.yml"])),
            Err(UsageError::Scope(ScopePatternError::EmptySegment))
        );
    }

    /// 🔴 2回書いたら**後勝ちにしない**。
    #[test]
    fn a_repeated_scope_is_an_error_not_an_overwrite() {
        assert_eq!(
            parse(&words(&["--scope", "/a", "--scope", "/b", "x", "f.yml"])),
            Err(UsageError::RepeatedScope)
        );
    }

    #[test]
    fn a_scope_without_a_pattern_is_an_error() {
        assert_eq!(
            parse(&words(&["x", "a.yml", "--scope"])),
            Err(UsageError::ScopeWithoutPattern)
        );
    }

    /// `--scope` の値は旗として読まない（`--` の前でも）。
    #[test]
    fn the_pattern_is_never_read_as_a_flag() {
        assert_eq!(
            parse(&words(&["--scope", "--json", "x", "a.yml"])),
            Err(UsageError::Scope(ScopePatternError::NotRooted))
        );
    }

    // ── パスの省略 ────────────────────────────────────────────────────────

    /// 🔴 パスを省略したら「今いる場所」（空のパス）。`.` とは別物である。
    #[test]
    fn an_omitted_path_becomes_the_current_place() {
        assert_eq!(paths(&["x"]), [PathBuf::new()]);
        assert_eq!(paths(&["x", "."]), [PathBuf::from(".")]);
        assert_eq!(paths(&["-i", "x"]), [PathBuf::new()]);
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

    /// needle が1つも無ければ使い方の誤りである（パスの省略は誤りではない）。
    #[test]
    fn a_run_without_a_needle_is_a_usage_error() {
        assert_eq!(parse(&words(&[])), Err(UsageError::Arguments));
        assert_eq!(parse(&words(&["--"])), Err(UsageError::Arguments));
    }

    #[test]
    fn an_unknown_flag_is_a_usage_error() {
        assert_eq!(
            parse(&words(&["--show-scope", "x", "a.yml"])),
            Err(UsageError::Arguments)
        );
        assert_eq!(
            parse(&words(&["-n", "x", "a.yml"])),
            Err(UsageError::Arguments)
        );
        assert_eq!(
            parse(&words(&["--scope=/a", "x", "a.yml"])),
            Err(UsageError::Arguments),
            "`--scope=<pattern>` は受けない"
        );
    }
}
