//! 照合が大文字小文字をどう扱うか。

/// 照合が大文字小文字をどう扱うか。**閉じた選択肢なので enum で表す**（RS-002）。
///
/// 🔴 [`CaseMatch::Fold`] でも**位置は原文で数える**。小文字化した文字列の上で
/// 位置を数えると、`İ`（小文字は `i` ＋合成用の点の2文字）のように
/// **長さが変わる文字を含む行だけ列がずれる**。だからここでは文字列を作らず、
/// 1文字ずつ [`char::to_lowercase`] の並びを比べる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMatch {
    /// 大文字小文字を区別する（既定）。
    Exact,
    /// 大文字小文字を無視する。
    Fold,
}

impl CaseMatch {
    /// `text` の中で `needle` が始まる位置を、**先頭からの文字数**で返す。
    ///
    /// 含まなければ `None`。空の `needle` は先頭（0 文字目）に当たる。
    pub(crate) fn locate(self, text: &str, needle: &str) -> Option<usize> {
        match self {
            Self::Exact => {
                let index = text.find(needle)?;
                Some(text.get(..index).unwrap_or("").chars().count())
            }
            Self::Fold => locate_folded(text, needle),
        }
    }
}

/// 大文字小文字を無視して `needle` の開始位置（先頭からの文字数）を探す。
///
/// 🔑 原文を1文字ずつ削りながら「ここから始まるか」を試す。
/// 小文字化した文字列を作らないので、**位置は常に原文の文字数**である。
fn locate_folded(text: &str, needle: &str) -> Option<usize> {
    let mut before = 0_usize;
    let mut rest = text;
    loop {
        if starts_folded(rest, needle) {
            return Some(before);
        }
        let mut remaining = rest.chars();
        remaining.next()?;
        rest = remaining.as_str();
        before = before.saturating_add(1_usize);
    }
}

/// `text` が `needle` で始まるか（大文字小文字を無視して）。
///
/// 比べるのは**1文字ずつ**である。`ß` と `ss` のように文字数が変わる対応は
/// 一致にしない。位置を返す道具なので、**長さが変わらないことを優先する**。
fn starts_folded(text: &str, needle: &str) -> bool {
    let mut found = text.chars();
    needle.chars().all(|wanted| {
        found
            .next()
            .is_some_and(|actual| actual.to_lowercase().eq(wanted.to_lowercase()))
    })
}

#[cfg(test)]
mod tests {
    use super::{CaseMatch, locate_folded, starts_folded};

    #[test]
    fn exact_counts_characters_not_bytes() {
        assert_eq!(CaseMatch::Exact.locate("あいう x", "x"), Some(4_usize));
        assert_eq!(CaseMatch::Exact.locate("abc", "B"), None);
    }

    #[test]
    fn fold_ignores_case_in_both_directions() {
        assert_eq!(CaseMatch::Fold.locate("abc", "B"), Some(1_usize));
        assert_eq!(CaseMatch::Fold.locate("ABC", "bc"), Some(1_usize));
        assert_eq!(CaseMatch::Fold.locate("abc", "z"), None);
    }

    /// 🔴 位置は**原文の文字数**である。`İ` の小文字は2文字なので、
    /// 小文字化した文字列の上で数えると 1 ずれる。
    #[test]
    fn fold_counts_the_characters_of_the_original_text() {
        assert_eq!(
            CaseMatch::Fold.locate("İstanbul Ziel", "ziel"),
            Some(9_usize)
        );
        assert_eq!(CaseMatch::Fold.locate("STRAßE Ziel", "ZIEL"), Some(7_usize));
    }

    /// 文字数が変わる対応は一致にしない（`ß` は `ss` に当たらない）。
    #[test]
    fn fold_does_not_expand_a_character_into_two() {
        assert_eq!(CaseMatch::Fold.locate("STRAßE", "ss"), None);
    }

    #[test]
    fn an_empty_needle_matches_at_the_start() {
        assert_eq!(CaseMatch::Exact.locate("abc", ""), Some(0_usize));
        assert_eq!(CaseMatch::Fold.locate("abc", ""), Some(0_usize));
        assert_eq!(CaseMatch::Fold.locate("", ""), Some(0_usize));
    }

    #[test]
    fn a_needle_longer_than_the_text_never_matches() {
        assert_eq!(locate_folded("ab", "abc"), None);
        assert!(!starts_folded("ab", "abc"));
    }
}
