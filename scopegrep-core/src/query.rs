//! 1回の検索を決める全て。

use alloc::borrow::ToOwned;
use alloc::string::String;

use crate::case_match::CaseMatch;
use crate::scope_path::ScopePath;
use crate::scope_pattern::ScopePattern;
use crate::search_scope::SearchScope;

/// 何を・どう探すか。**探し方の条件は1つの型に束ねる**。
///
/// 🔑 条件が増えるたびに [`crate::document::Document::search`] の引数が増える形にしない。
/// 引数が増えると、呼ぶ側が**順番を取り違えても型が助けてくれない**位置が増える。
///
/// 既定（[`Query::new`] だけ）は「値だけ・大文字小文字を区別・絞り込み無し」で、
/// **広げる方向にしか動かない**。旗を付けなければ、この道具は何も余計に返さない。
///
/// ```
/// use scopegrep_core::query::Query;
///
/// let source = "steps:\n  - name: Build\n    run: NPM ci\n";
/// let Ok(document) = scopegrep_core::parse(source) else {
///     return;
/// };
/// assert_eq!(document.search(&Query::new("npm")).len(), 0);
/// assert_eq!(document.search(&Query::new("npm").ignoring_case()).len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    needle: String,
    case: CaseMatch,
    kinds: SearchScope,
    within: Option<ScopePattern>,
}

impl Query {
    /// 固定文字列を探す条件を作る（正規表現ではない）。
    #[must_use]
    pub fn new(needle: &str) -> Self {
        Self {
            needle: needle.to_owned(),
            case: CaseMatch::Exact,
            kinds: SearchScope::Values,
            within: None,
        }
    }

    /// 大文字小文字を無視して照合する。**列は原文の一致位置のまま**である。
    #[must_use]
    pub fn ignoring_case(self) -> Self {
        Self {
            case: CaseMatch::Fold,
            ..self
        }
    }

    /// コメント内の一致も返す（種別は [`crate::hit::Hit::kind`] で区別できる）。
    #[must_use]
    pub fn including_comments(self) -> Self {
        Self {
            kinds: SearchScope::ValuesAndComments,
            ..self
        }
    }

    /// 所属が `pattern` に一致するものだけを返す。
    #[must_use]
    pub fn within(self, pattern: ScopePattern) -> Self {
        Self {
            within: Some(pattern),
            ..self
        }
    }

    /// 探す固定文字列。
    pub(crate) fn needle(&self) -> &str {
        &self.needle
    }

    /// 大文字小文字の扱い。
    pub(crate) fn case(&self) -> CaseMatch {
        self.case
    }

    /// 値だけを探すか、コメントも探すか。
    pub(crate) fn kinds(&self) -> SearchScope {
        self.kinds
    }

    /// この所属が絞り込みに入っているか。絞り込みが無ければ常に真。
    pub(crate) fn covers(&self, path: &ScopePath) -> bool {
        self.within
            .as_ref()
            .is_none_or(|pattern| pattern.matches(path))
    }
}

#[cfg(test)]
mod tests {
    use super::Query;
    use crate::case_match::CaseMatch;
    use crate::scope_path::ScopePath;
    use crate::scope_pattern::ScopePattern;
    use crate::search_scope::SearchScope;
    use crate::segment::Segment;
    use alloc::borrow::ToOwned;
    use alloc::vec;

    #[test]
    fn a_plain_query_is_the_narrowest_one() {
        let query = Query::new("x");
        assert_eq!(query.needle(), "x");
        assert_eq!(query.case(), CaseMatch::Exact);
        assert_eq!(query.kinds(), SearchScope::Values);
        assert!(query.covers(&ScopePath::new(vec![])), "絞り込みが無い");
    }

    #[test]
    fn each_builder_widens_exactly_one_thing() {
        let query = Query::new("x").ignoring_case().including_comments();
        assert_eq!(query.case(), CaseMatch::Fold);
        assert_eq!(query.kinds(), SearchScope::ValuesAndComments);
        assert_eq!(query.needle(), "x");
    }

    #[test]
    fn a_scope_pattern_narrows_by_place() {
        let Ok(pattern) = ScopePattern::parse("/jobs/*") else {
            panic!("読めるはず");
        };
        let query = Query::new("x").within(pattern);
        let inside = ScopePath::new(vec![
            Segment::Key("jobs".to_owned()),
            Segment::Key("e2e".to_owned()),
        ]);
        let outside = ScopePath::new(vec![Segment::Key("jobs".to_owned())]);
        assert!(query.covers(&inside));
        assert!(!query.covers(&outside));
    }
}
