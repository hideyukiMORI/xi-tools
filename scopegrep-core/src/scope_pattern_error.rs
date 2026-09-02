//! 所属パターンが読めなかった理由。

use core::error::Error;
use core::fmt;

/// 所属パターンが読めなかった理由。**閉じた選択肢なので enum で表す**（RS-002）。
///
/// 🔴 「読めない」で終わらせず、**何が悪いか**を必ず言う。
/// パターンは人が手で打つものなので、直し方が分からない報告は報告ではない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopePatternError {
    /// 空のパターン。
    Empty,
    /// 先頭が `/` でない。
    NotRooted,
    /// 空のセグメントがある（`//` や末尾の `/`）。
    EmptySegment,
}

impl fmt::Display for ScopePatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Empty => f.write_str("the pattern is empty"),
            Self::NotRooted => f.write_str("the pattern must start with `/`"),
            Self::EmptySegment => f.write_str(
                "the pattern has an empty segment (`//` and a trailing `/` cannot be written)",
            ),
        }
    }
}

impl Error for ScopePatternError {}
