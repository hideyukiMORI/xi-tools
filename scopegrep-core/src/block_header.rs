//! ブロックスカラーの指示子（`|` / `>` の後ろ）。

use crate::malformed_input::MalformedInput;
use crate::parse_error_kind::ParseErrorKind;

/// ブロックスカラーの指示子から読み取れたもの。
///
/// チョンピング（`+` / `-`）は**読み飛ばす**。この道具は内容を復元せず、
/// 各行をそのまま持つだけなので、末尾改行の扱いは結果に影響しない。
#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockHeader {
    indent: Option<usize>,
}

impl BlockHeader {
    /// インデント指示子（`|2` の `2`）から作る。
    pub(crate) fn new(indent: Option<usize>) -> Self {
        Self { indent }
    }

    /// 明示されたインデント幅（親からの相対）。
    pub(crate) fn indent(self) -> Option<usize> {
        self.indent
    }
}

/// `|` / `>` で始まる指示子を読む。
///
/// # Errors
///
/// 指示子が2つ以上あるときや、知らない文字が続くときはエラーにする。
pub(crate) fn parse(text: &str) -> Result<BlockHeader, ParseErrorKind> {
    let mut indent: Option<usize> = None;
    let mut chomping = false;
    for (position, c) in text.char_indices().skip(1_usize) {
        if c.is_whitespace() {
            return finish(text, position, indent);
        }
        if let Some(digit) = c.to_digit(10_u32) {
            indent = accept_indent(digit, indent)?;
            continue;
        }
        if c == '+' || c == '-' {
            if chomping {
                return Err(malformed());
            }
            chomping = true;
            continue;
        }
        return Err(malformed());
    }
    Ok(BlockHeader::new(indent))
}

/// インデント指示子を1つだけ受け取る。0 は幅として無効である。
fn accept_indent(digit: u32, seen: Option<usize>) -> Result<Option<usize>, ParseErrorKind> {
    if digit == 0_u32 || seen.is_some() {
        return Err(malformed());
    }
    let width = usize::try_from(digit).ok().ok_or_else(malformed)?;
    Ok(Some(width))
}

/// 指示子の後ろは空白かコメントだけを許す。
fn finish(
    text: &str,
    position: usize,
    indent: Option<usize>,
) -> Result<BlockHeader, ParseErrorKind> {
    let tail = text.get(position..).unwrap_or("").trim();
    if tail.is_empty() || tail.starts_with('#') {
        Ok(BlockHeader::new(indent))
    } else {
        Err(malformed())
    }
}

/// この場所のエラーは1種類しかない。
fn malformed() -> ParseErrorKind {
    ParseErrorKind::Malformed(MalformedInput::BlockScalarHeader)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn bare_block_has_no_explicit_indent() {
        assert_eq!(parse("|").expect("|").indent(), None);
    }

    #[test]
    fn chomping_and_indent_are_read() {
        assert_eq!(parse("|2-").expect("|2-").indent(), Some(2_usize));
        assert_eq!(parse(">-").expect(">-").indent(), None);
    }

    #[test]
    fn a_comment_may_follow() {
        assert_eq!(parse("| # note").expect("| # note").indent(), None);
    }

    #[test]
    fn unknown_indicator_is_rejected() {
        assert!(parse("|x").is_err());
        assert!(parse("|22").is_err());
    }
}
