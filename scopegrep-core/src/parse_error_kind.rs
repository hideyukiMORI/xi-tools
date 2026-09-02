//! 読めなかった理由の種別。

use core::fmt;

use crate::malformed_input::MalformedInput;
use crate::unsupported_syntax::UnsupportedSyntax;

/// 読めなかった理由。**「読める部分集合の外」と「壊れた入力」を分ける**。
///
/// 前者は道具の守備範囲の話で、後者は入力の話である。使う側の対応が違うので混ぜない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// 読める部分集合の外にある構文。
    Unsupported(UnsupportedSyntax),
    /// 構文としておかしい入力。
    Malformed(MalformedInput),
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unsupported(syntax) => write!(f, "{syntax} は読めない構文である"),
            Self::Malformed(input) => write!(f, "{input}"),
        }
    }
}
