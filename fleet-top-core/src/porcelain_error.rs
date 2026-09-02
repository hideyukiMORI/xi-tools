//! `git status --porcelain=v2` を読めなかったことの報告。

use core::error::Error;
use core::fmt;

use crate::porcelain_error_kind::PorcelainErrorKind;

/// porcelain v2 を読めなかったことの報告。**必ず行番号と種別を持つ**。
///
/// 行番号は 1 起点で、**0 は「入力全体」**を意味する
/// （[`PorcelainErrorKind::MissingHead`] のように、特定の行の問題ではないもの）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PorcelainError {
    kind: PorcelainErrorKind,
    line: u32,
}

impl PorcelainError {
    /// 種別と行番号から作る。行番号は 1 起点で、0 は入力全体を指す。
    pub(crate) fn new(kind: PorcelainErrorKind, line: u32) -> Self {
        Self { kind, line }
    }

    /// 読めなかった理由。
    #[must_use]
    pub fn kind(&self) -> &PorcelainErrorKind {
        &self.kind
    }

    /// 読めなかった行（1 起点）。0 は入力全体を指す。
    #[must_use]
    pub fn line(&self) -> u32 {
        self.line
    }
}

impl fmt::Display for PorcelainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            0_u32 => write!(f, "{}", self.kind),
            number => write!(f, "{number}行目: {}", self.kind),
        }
    }
}

impl Error for PorcelainError {}

#[cfg(test)]
mod tests {
    use super::PorcelainError;
    use crate::porcelain_error_kind::PorcelainErrorKind;
    use alloc::format;

    #[test]
    fn display_names_the_line_and_the_reason() {
        let error = PorcelainError::new(PorcelainErrorKind::MalformedHeader, 4_u32);
        assert_eq!(format!("{error}"), "4行目: `#` 見出しの値が読めない");
    }

    /// 行 0 は入力全体を指すので、行番号を出さない。
    #[test]
    fn display_omits_the_line_when_the_whole_input_is_at_fault() {
        let error = PorcelainError::new(PorcelainErrorKind::MissingHead, 0_u32);
        assert_eq!(
            format!("{error}"),
            "`# branch.head` が無い（`--branch` を付けて実行する）"
        );
    }

    #[test]
    fn keeps_the_kind_and_the_line() {
        let error = PorcelainError::new(PorcelainErrorKind::UnexpectedLine, 2_u32);
        assert_eq!(error.kind(), &PorcelainErrorKind::UnexpectedLine);
        assert_eq!(error.line(), 2_u32);
    }

    /// `core::error::Error` として扱える（bin 側で `dyn Error` に載せるため）。
    #[test]
    fn is_a_std_error() {
        let error = PorcelainError::new(PorcelainErrorKind::MissingHead, 0_u32);
        let raised: &dyn core::error::Error = &error;
        assert!(!format!("{raised}").is_empty());
    }
}
