//! 1回の検索を決める全て。

use alloc::boxed::Box;
use alloc::rc::Rc;

use crate::case_match::CaseMatch;
use crate::fixed_string::FixedString;
use crate::matcher::Matcher;
use crate::matching::Matching;
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
    matching: Matching,
    kinds: SearchScope,
    within: Option<ScopePattern>,
}

impl Query {
    /// 固定文字列を探す条件を作る（正規表現ではない）。
    #[must_use]
    pub fn new(needle: &str) -> Self {
        Self {
            matching: Matching::Fixed(FixedString::new(needle, CaseMatch::Exact)),
            kinds: SearchScope::Values,
            within: None,
        }
    }

    /// 外から渡した [`Matcher`] で探す条件を作る。
    ///
    /// 🔑 中核は正規表現を知らない。**照合そのものを差し込む口がここである**
    /// （ADR 0002。`scopegrep-core` は `no_std`・依存 0 のまま）。
    #[must_use]
    pub fn with_matcher(matcher: Box<dyn Matcher>) -> Self {
        Self {
            matching: Matching::Custom(Rc::from(matcher)),
            kinds: SearchScope::Values,
            within: None,
        }
    }

    /// 大文字小文字を無視して照合する。**列は原文の一致位置のまま**である。
    ///
    /// 🔴 効くのは [`Query::new`] で作った固定文字列の条件だけである。
    /// [`Query::with_matcher`] で渡した照合は**そのまま**で、
    /// 大文字小文字の扱いは [`Matcher`] 自身が決める（正規表現なら組み立て時に決める）。
    #[must_use]
    pub fn ignoring_case(self) -> Self {
        let matching = match self.matching {
            Matching::Fixed(needle) => Matching::Fixed(needle.folded()),
            Matching::Custom(matcher) => Matching::Custom(matcher),
        };
        Self {
            matching,
            kinds: self.kinds,
            within: self.within,
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

    /// 一致を判定するもの。
    pub(crate) fn matcher(&self) -> &dyn Matcher {
        &self.matching
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
    use crate::matcher::Matcher;
    use crate::scope_path::ScopePath;
    use crate::scope_pattern::ScopePattern;
    use crate::search_scope::SearchScope;
    use crate::segment::Segment;
    use alloc::borrow::ToOwned;
    use alloc::boxed::Box;
    use alloc::vec;

    /// 語尾が `x` の行にだけ当たる、テスト用の照合。
    #[derive(Debug)]
    struct LastCharacter;

    impl Matcher for LastCharacter {
        fn find(&self, text: &str) -> Option<usize> {
            text.chars()
                .count()
                .checked_sub(1_usize)
                .filter(|_| text.ends_with('x'))
        }
    }

    #[test]
    fn a_plain_query_is_the_narrowest_one() {
        let query = Query::new("x");
        assert_eq!(query.matcher().find("a x"), Some(2_usize));
        assert_eq!(
            query.matcher().find("a X"),
            None,
            "既定は大文字小文字を区別する"
        );
        assert_eq!(query.kinds(), SearchScope::Values);
        assert!(query.covers(&ScopePath::new(vec![])), "絞り込みが無い");
    }

    #[test]
    fn each_builder_widens_exactly_one_thing() {
        let query = Query::new("x").ignoring_case().including_comments();
        assert_eq!(query.matcher().find("a X"), Some(2_usize));
        assert_eq!(query.kinds(), SearchScope::ValuesAndComments);
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

    /// 渡した [`Matcher`] がそのまま照合に使われる。
    #[test]
    fn a_given_matcher_decides_the_match() {
        let query = Query::with_matcher(Box::new(LastCharacter));
        assert_eq!(query.matcher().find("あいうx"), Some(3_usize));
        assert_eq!(query.matcher().find("abc"), None);
        assert_eq!(query.kinds(), SearchScope::Values, "既定は変わらない");
    }

    /// 🔴 `ignoring_case` は**固定文字列にしか効かない**。
    /// 渡された照合の大文字小文字の扱いは、その照合自身が決める。
    #[test]
    fn ignoring_case_leaves_a_given_matcher_alone() {
        let query = Query::with_matcher(Box::new(LastCharacter));
        let folded = query.clone().ignoring_case();
        assert_eq!(folded.matcher().find("aX"), None, "照合は変わっていない");
        assert_eq!(folded, query, "同じ照合を指したままである");
    }

    /// 条件どうしは比べられる。固定文字列は中身で、渡された照合は同一性で。
    #[test]
    fn two_queries_are_equal_when_they_search_the_same_way() {
        assert_eq!(Query::new("x"), Query::new("x"));
        assert_ne!(Query::new("x"), Query::new("y"));
        assert_ne!(Query::new("x"), Query::new("x").ignoring_case());
        assert_ne!(
            Query::with_matcher(Box::new(LastCharacter)),
            Query::with_matcher(Box::new(LastCharacter)),
            "別に組んだ照合は等しくない"
        );
    }
}
