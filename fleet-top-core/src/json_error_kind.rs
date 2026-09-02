//! JSON を読めなかった理由の種別。

use core::fmt;

/// JSON を読めなかった理由。
///
/// 🔑 「読めなかった」で終わらせず、**何が起きたか**を言う。GraphQL の応答が
/// 読めないとき、それが切れた応答なのか・想定外の文字なのか・数の書き方なのかで
/// 使う側の対応が違う（RS-002）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonErrorKind {
    /// 入力が値の途中で尽きた。
    UnexpectedEnd,
    /// 文法上ここに来られない文字が現れた。
    UnexpectedCharacter(char),
    /// `\` の後が JSON のエスケープ文字でない。
    InvalidEscape,
    /// `\u` の後が 16 進 4 桁でない、または孤立サロゲートである。
    InvalidUnicodeEscape,
    /// エスケープされていない制御文字（U+0000〜U+001F）が文字列に現れた。
    ControlCharacterInString,
    /// 数の書き方が RFC 8259 の文法に合わない（`01` / `.5` / `1.` / `1e` / `-`）。
    InvalidNumber,
    /// 上位の値 1 つを読み切った後に、空白以外が残っている。
    TrailingCharacters,
    /// 配列とオブジェクトの入れ子が上限を超えた。
    TooDeep,
}

impl fmt::Display for JsonErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnexpectedEnd => f.write_str("unexpected end of input"),
            Self::UnexpectedCharacter(character) => write!(f, "unexpected character `{character}`"),
            Self::InvalidEscape => f.write_str("`\\` is not followed by a JSON escape character"),
            Self::InvalidUnicodeEscape => {
                f.write_str("`\\u` is not 4 hex digits, or is a lone surrogate")
            }
            Self::ControlCharacterInString => {
                f.write_str("unescaped control character in a string")
            }
            Self::InvalidNumber => f.write_str("invalid number"),
            Self::TrailingCharacters => f.write_str("trailing characters after the value"),
            Self::TooDeep => f.write_str("nesting too deep"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JsonErrorKind;
    use alloc::format;

    #[test]
    fn display_names_the_offending_character() {
        let kind = JsonErrorKind::UnexpectedCharacter('x');
        assert_eq!(format!("{kind}"), "unexpected character `x`");
    }

    /// 全ての種別が空でない説明を持つ。**説明の無い種別を足せない**ようにする。
    #[test]
    fn every_kind_has_a_message() {
        let kinds = [
            JsonErrorKind::UnexpectedEnd,
            JsonErrorKind::UnexpectedCharacter('!'),
            JsonErrorKind::InvalidEscape,
            JsonErrorKind::InvalidUnicodeEscape,
            JsonErrorKind::ControlCharacterInString,
            JsonErrorKind::InvalidNumber,
            JsonErrorKind::TrailingCharacters,
            JsonErrorKind::TooDeep,
        ];
        for kind in kinds {
            assert!(!format!("{kind}").is_empty());
        }
    }
}
