//! 正規表現の照合。**feature `regex` を付けたときだけ在る**（ADR 0002）。
//!
//! 🔴 中核（`scopegrep-core`）ではなくここに置く。中核は `no_std`・依存 0 で、
//! 正規表現エンジンは std を要る。**依存が入る場所をバイナリ側に閉じる**ための配置である。

use regex::{Error, Regex, RegexBuilder};

use scopegrep_core::case_match::CaseMatch;
use scopegrep_core::matcher::Matcher;

/// 正規表現で1行を照合する [`Matcher`]。
///
/// 🔴 一致は**行単位**である（`^` `$` は行の先頭と末尾）。値を行ごとに持つ設計の
/// 帰結であって、複数行スカラーを跨いだ一致は起きない。
#[derive(Debug)]
pub(crate) struct RegexMatcher {
    pattern: Regex,
}

impl RegexMatcher {
    /// パターンと大文字小文字の扱いから組む。
    ///
    /// 🔑 `-i` は `(?i)` を前に足すのではなく [`RegexBuilder::case_insensitive`] で渡す。
    /// パターンの文字列に手を入れると、`(?-i)` を書いた人の意図を黙って壊す。
    ///
    /// # Errors
    ///
    /// パターンが読めなければ [`Error`] を返す。**黙って固定文字列に落とさない。**
    pub(crate) fn new(pattern: &str, case: CaseMatch) -> Result<Self, Error> {
        let mut builder = RegexBuilder::new(pattern);
        let configured = match case {
            CaseMatch::Exact => &mut builder,
            CaseMatch::Fold => builder.case_insensitive(true),
        };
        configured.build().map(|built| Self { pattern: built })
    }
}

impl Matcher for RegexMatcher {
    /// 🔴 `regex` が返すのは**バイト位置**である。桁は文字数で数える約束なので、
    /// ここで必ず変換する。変換を忘れると非 ASCII の行だけ桁がずれる。
    fn find(&self, text: &str) -> Option<usize> {
        let found = self.pattern.find(text)?;
        Some(text.get(..found.start()).unwrap_or("").chars().count())
    }
}

#[cfg(test)]
mod tests {
    use super::RegexMatcher;
    use scopegrep_core::case_match::CaseMatch;
    use scopegrep_core::matcher::Matcher;

    fn matcher(pattern: &str, case: CaseMatch) -> RegexMatcher {
        RegexMatcher::new(pattern, case).expect("読めるはず")
    }

    #[test]
    fn a_pattern_finds_the_first_match() {
        assert_eq!(
            matcher("cancel+ed\\(\\)", CaseMatch::Exact).find("${{ !cancelled() }}"),
            Some(5_usize)
        );
        assert_eq!(matcher("^npm", CaseMatch::Exact).find("run npm ci"), None);
    }

    /// 🔴 位置は**文字数**である。バイト位置のままだと、ここが 12 になる。
    #[test]
    fn a_position_is_counted_in_characters_not_bytes() {
        assert_eq!(
            matcher("Ziel", CaseMatch::Exact).find("あいう STRAßE Ziel"),
            Some(11_usize)
        );
    }

    #[test]
    fn case_insensitivity_comes_from_the_builder() {
        assert_eq!(matcher("ZIEL", CaseMatch::Exact).find("ziel"), None);
        assert_eq!(matcher("ZIEL", CaseMatch::Fold).find("ziel"), Some(0_usize));
    }

    /// パターンの中で明示的に打ち消した指定は、`-i` に上書きされない。
    #[test]
    fn an_explicit_marker_inside_the_pattern_still_wins() {
        assert_eq!(matcher("(?-i)ZIEL", CaseMatch::Fold).find("ziel"), None);
    }

    #[test]
    fn a_broken_pattern_is_an_error() {
        assert!(
            RegexMatcher::new("cancel+ed(", CaseMatch::Exact).is_err(),
            "閉じない括弧は読めない"
        );
        assert!(
            RegexMatcher::new("cancel+ed\\(", CaseMatch::Exact).is_ok(),
            "退避した括弧はただの文字である"
        );
    }
}
