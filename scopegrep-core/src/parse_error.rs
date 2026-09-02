//! 読めなかったことの報告。

use core::error::Error;
use core::fmt;

use crate::line_number::LineNumber;
use crate::parse_error_kind::ParseErrorKind;

/// 読めなかったことの報告。**必ず行番号と種別を持つ**。
///
/// 「このファイルは読めない」で終わらせず、「何行目の何が読めなかったか」を言う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    line: LineNumber,
    kind: ParseErrorKind,
}

impl ParseError {
    /// 行番号と種別から作る。
    pub(crate) fn new(line: LineNumber, kind: ParseErrorKind) -> Self {
        Self { line, kind }
    }

    /// 読めなかった行。
    #[must_use]
    pub fn line(&self) -> LineNumber {
        self.line
    }

    /// 読めなかった理由。
    #[must_use]
    pub fn kind(&self) -> ParseErrorKind {
        self.kind
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}行目: {}", self.line, self.kind)
    }
}

impl Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::ParseError;
    use crate::line_number::LineNumber;
    use crate::malformed_input::MalformedInput;
    use crate::parse_error_kind::ParseErrorKind;
    use alloc::format;

    #[test]
    fn display_names_the_line_and_the_reason() {
        let line = LineNumber::new(7_u32).expect("7 は行番号である");
        let error = ParseError::new(
            line,
            ParseErrorKind::Malformed(MalformedInput::TabIndentation),
        );
        assert_eq!(format!("{error}"), "7行目: インデントにタブが混じっている");
    }
}
