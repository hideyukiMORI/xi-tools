//! 固定文字列の照合。

use alloc::borrow::ToOwned;
use alloc::string::String;

use crate::case_match::CaseMatch;
use crate::matcher::Matcher;

/// 固定文字列を探す [`Matcher`]。**これがこの道具の既定である**（正規表現ではない）。
///
/// 位置は常に**原文の文字数**で数える（[`CaseMatch`] の doc を見よ）。
///
/// ```
/// use scopegrep_core::case_match::CaseMatch;
/// use scopegrep_core::fixed_string::FixedString;
/// use scopegrep_core::matcher::Matcher;
///
/// assert_eq!(FixedString::new("x", CaseMatch::Exact).find("あいう x"), Some(4));
/// assert_eq!(FixedString::new("X", CaseMatch::Exact).find("あいう x"), None);
/// assert_eq!(FixedString::new("X", CaseMatch::Fold).find("あいう x"), Some(4));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedString {
    needle: String,
    case: CaseMatch,
}

impl FixedString {
    /// 探す固定文字列と、大文字小文字の扱いから作る。
    #[must_use]
    pub fn new(needle: &str, case: CaseMatch) -> Self {
        Self {
            needle: needle.to_owned(),
            case,
        }
    }

    /// 大文字小文字を無視する版を返す。
    pub(crate) fn folded(&self) -> Self {
        Self {
            needle: self.needle.clone(),
            case: CaseMatch::Fold,
        }
    }
}

impl Matcher for FixedString {
    fn find(&self, text: &str) -> Option<usize> {
        self.case.locate(text, &self.needle)
    }
}

#[cfg(test)]
mod tests {
    use super::FixedString;
    use crate::case_match::CaseMatch;
    use crate::matcher::Matcher;

    #[test]
    fn an_exact_needle_counts_characters_not_bytes() {
        assert_eq!(
            FixedString::new("x", CaseMatch::Exact).find("あいう x"),
            Some(4_usize)
        );
    }

    #[test]
    fn an_exact_needle_distinguishes_case() {
        assert_eq!(FixedString::new("B", CaseMatch::Exact).find("abc"), None);
        assert_eq!(
            FixedString::new("B", CaseMatch::Fold).find("abc"),
            Some(1_usize)
        );
    }

    /// 空の needle は先頭に当たる（`--scope` だけで引くときの形）。
    #[test]
    fn an_empty_needle_matches_at_the_start() {
        assert_eq!(
            FixedString::new("", CaseMatch::Exact).find("abc"),
            Some(0_usize)
        );
    }

    /// [`FixedString::folded`] は needle を変えず、扱いだけを緩める。
    #[test]
    fn folding_widens_only_the_case() {
        let exact = FixedString::new("ziel", CaseMatch::Exact);
        assert_eq!(exact.find("İstanbul Ziel"), None);
        assert_eq!(exact.folded().find("İstanbul Ziel"), Some(9_usize));
        assert_eq!(exact.folded(), FixedString::new("ziel", CaseMatch::Fold));
    }
}
