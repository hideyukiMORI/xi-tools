//! JSON を読めなかったことの報告。

use core::error::Error;
use core::fmt;

use crate::json_error_kind::JsonErrorKind;

/// JSON を読めなかったことの報告。**必ず位置と種別を持つ**。
///
/// 位置は**先頭からの文字数**（`char` の数・0 起点）である。バイト数にしない理由は、
/// 応答に日本語が入ったときに人が数えられる位置でなくなるからで、
/// この道具の失敗はそのまま人に見せる（設計メモ「出力の形」の `?` 行の理由）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    offset: usize,
    kind: JsonErrorKind,
}

impl JsonError {
    /// 位置と種別から作る。位置は先頭からの文字数（0 起点）。
    pub(crate) fn new(offset: usize, kind: JsonErrorKind) -> Self {
        Self { offset, kind }
    }

    /// 読めなかった位置。**先頭からの文字数**（`char` の数・0 起点）。
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// 読めなかった理由。
    #[must_use]
    pub fn kind(&self) -> &JsonErrorKind {
        &self.kind
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at character {}: {}", self.offset, self.kind)
    }
}

impl Error for JsonError {}

#[cfg(test)]
mod tests {
    use super::JsonError;
    use crate::json_error_kind::JsonErrorKind;
    use alloc::format;

    #[test]
    fn display_names_the_offset_and_the_reason() {
        let error = JsonError::new(12_usize, JsonErrorKind::TrailingCharacters);
        assert_eq!(
            format!("{error}"),
            "at character 12: trailing characters after the value"
        );
    }

    #[test]
    fn keeps_the_offset_and_the_kind() {
        let error = JsonError::new(3_usize, JsonErrorKind::UnexpectedCharacter('z'));
        assert_eq!(error.offset(), 3_usize);
        assert_eq!(error.kind(), &JsonErrorKind::UnexpectedCharacter('z'));
    }

    /// `core::error::Error` として扱える（bin 側で `dyn Error` に載せるため）。
    #[test]
    fn is_a_std_error() {
        let error = JsonError::new(0_usize, JsonErrorKind::UnexpectedEnd);
        let raised: &dyn core::error::Error = &error;
        assert!(!format!("{raised}").is_empty());
    }
}
