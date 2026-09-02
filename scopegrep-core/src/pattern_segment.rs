//! 所属パターンの1要素。

use alloc::string::String;

use crate::segment::Segment;

/// 所属パターンの1要素。
///
/// 🔑 `*` は**ちょうど1セグメント**、`**` は**0 個以上**である。
/// 部分一致のグロブ（`ste*`）は用意しない。**一つの事に一つの意味**を保つためで、
/// 「`*` がどこまで食うか」を場所ごとに考えなくてよくする（RS-002）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PatternSegment {
    /// 生のキー／索引と完全一致する語。
    Literal(String),
    /// ちょうど1セグメント（`*`）。
    Any,
    /// 0 個以上のセグメント（`**`）。
    AnyDepth,
}

impl PatternSegment {
    /// 参照トークン1つを読む。
    ///
    /// リテラルは RFC 6901 のエスケープ（`~1`→`/`、`~0`→`~`）を解いてから持つ。
    /// **順序を逆にしない**（先に `~0` を解くと `~01` が `/` になる）。
    pub(crate) fn read(token: &str) -> Self {
        match token {
            "**" => Self::AnyDepth,
            "*" => Self::Any,
            literal => Self::Literal(literal.replace("~1", "/").replace("~0", "~")),
        }
    }

    /// この要素が、所属パスの1要素に当たるか。
    ///
    /// `AnyDepth` は 0 個以上を表すので、1要素との突き合わせには使わない
    /// （呼び手が先に取り除く）。
    pub(crate) fn accepts(&self, segment: &Segment) -> bool {
        match *self {
            Self::Literal(ref text) => segment.is_written(text),
            Self::Any | Self::AnyDepth => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PatternSegment;
    use alloc::borrow::ToOwned;

    #[test]
    fn stars_are_the_only_two_special_tokens() {
        assert_eq!(PatternSegment::read("**"), PatternSegment::AnyDepth);
        assert_eq!(PatternSegment::read("*"), PatternSegment::Any);
        assert_eq!(
            PatternSegment::read("steps"),
            PatternSegment::Literal("steps".to_owned())
        );
    }

    /// 🔑 部分一致のグロブは持たない。`ste*` はそのままのリテラルである。
    #[test]
    fn a_star_inside_a_word_is_literal() {
        assert_eq!(
            PatternSegment::read("ste*"),
            PatternSegment::Literal("ste*".to_owned())
        );
    }

    #[test]
    fn escapes_are_undone_in_the_right_order() {
        assert_eq!(
            PatternSegment::read("a~1b"),
            PatternSegment::Literal("a/b".to_owned())
        );
        assert_eq!(
            PatternSegment::read("a~0b"),
            PatternSegment::Literal("a~b".to_owned())
        );
        assert_eq!(
            PatternSegment::read("~01"),
            PatternSegment::Literal("~1".to_owned())
        );
    }
}
