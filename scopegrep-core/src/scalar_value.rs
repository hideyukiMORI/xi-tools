//! 1行スカラーの読み取り。

use alloc::borrow::ToOwned;
use alloc::string::String;

use crate::column::Column;
use crate::malformed_input::MalformedInput;
use crate::parse_error_kind::ParseErrorKind;
use crate::unsupported_syntax::UnsupportedSyntax;

/// 読み取ったスカラー値と、その先頭の桁。
///
/// 値は**原文のまま**持つ（クォートの中身をエスケープ解除しない）。
/// 人が `grep` で見る文字列と同じものに当たることを優先する（設計メモ「検索の意味」）。
#[derive(Debug, Clone)]
pub(crate) struct ScalarValue {
    text: String,
    column: Column,
}

impl ScalarValue {
    /// 値と桁から作る。
    pub(crate) fn new(text: String, column: Column) -> Self {
        Self { text, column }
    }

    /// 原文のままの値。
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// 値の先頭の桁。
    pub(crate) fn column(&self) -> Column {
        self.column
    }

    /// 値を取り出す。
    pub(crate) fn into_text(self) -> String {
        self.text
    }
}

/// `line` の `start` バイト位置から始まる1行スカラーを読む。
///
/// # Errors
///
/// アンカー・エイリアス・タグ・行内で閉じないクォートやフロー記法はエラーにする。
pub(crate) fn parse(line: &str, start: usize) -> Result<ScalarValue, ParseErrorKind> {
    let rest = line.get(start..).unwrap_or("");
    let column = Column::after(line.get(..start).unwrap_or("").chars().count());
    let Some(first) = rest.chars().next() else {
        return Ok(ScalarValue::new(String::new(), column));
    };
    match first {
        '&' => Err(ParseErrorKind::Unsupported(UnsupportedSyntax::Anchor)),
        '*' => Err(ParseErrorKind::Unsupported(UnsupportedSyntax::Alias)),
        '!' => Err(ParseErrorKind::Unsupported(UnsupportedSyntax::Tag)),
        '"' | '\'' => bounded(
            rest,
            scan_quoted(rest, first),
            column,
            UnsupportedSyntax::MultiLineScalar,
        ),
        '[' | '{' => bounded(
            rest,
            scan_flow(rest),
            column,
            UnsupportedSyntax::MultiLineFlow,
        ),
        _ => Ok(ScalarValue::new(plain(rest), column)),
    }
}

/// 閉じ位置が分かっている値を切り出す。閉じていなければ `missing` を返す。
fn bounded(
    rest: &str,
    end: Option<usize>,
    column: Column,
    missing: UnsupportedSyntax,
) -> Result<ScalarValue, ParseErrorKind> {
    let Some(end) = end else {
        return Err(ParseErrorKind::Unsupported(missing));
    };
    let tail = rest.get(end..).unwrap_or("").trim();
    if !tail.is_empty() && !tail.starts_with('#') {
        return Err(ParseErrorKind::Malformed(MalformedInput::TrailingContent));
    }
    Ok(ScalarValue::new(
        rest.get(..end).unwrap_or("").to_owned(),
        column,
    ))
}

/// プレーンスカラー。**空白の直後の `#` から行末まで**はコメントなので落とす。
fn plain(rest: &str) -> String {
    let mut cut = rest.len();
    let mut previous: Option<char> = None;
    for (index, c) in rest.char_indices() {
        if c == '#' && previous.is_none_or(char::is_whitespace) {
            cut = index;
            break;
        }
        previous = Some(c);
    }
    rest.get(..cut).unwrap_or("").trim_end().to_owned()
}

/// クォートの閉じ位置（閉じクォートの次のバイト位置）を返す。
///
/// `"…"` は `\` エスケープ、`'…'` は `''` エスケープを見る。
/// 行内で閉じなければ `None`（＝複数行のクォート）。
pub(crate) fn scan_quoted(rest: &str, quote: char) -> Option<usize> {
    let mut chars = rest.char_indices().peekable();
    chars.next()?;
    loop {
        let (index, c) = chars.next()?;
        if c == '\\' && quote == '"' {
            chars.next()?;
            continue;
        }
        if c != quote {
            continue;
        }
        if quote == '\'' && chars.peek().is_some_and(|&(_, next)| next == '\'') {
            chars.next()?;
            continue;
        }
        return Some(index.saturating_add(c.len_utf8()));
    }
}

/// フロー記法の閉じ位置（閉じ括弧の次のバイト位置）を返す。
///
/// **中には入らない**。1つのスカラーとして原文のまま持つ（設計メモの部分集合）。
fn scan_flow(rest: &str) -> Option<usize> {
    let mut depth = 0_usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, c) in rest.char_indices() {
        if let Some(open) = quote {
            quote = inside_quote(open, c, &mut escaped);
            continue;
        }
        match c {
            '\'' | '"' => quote = Some(c),
            '[' | '{' => depth = depth.saturating_add(1_usize),
            ']' | '}' => {
                depth = depth.saturating_sub(1_usize);
                if depth == 0_usize {
                    return Some(index.saturating_add(1_usize));
                }
            }
            _ => {}
        }
    }
    None
}

/// クォートの内側で1文字進む。閉じたら `None` を返す。
fn inside_quote(open: char, c: char, escaped: &mut bool) -> Option<char> {
    if *escaped {
        *escaped = false;
        return Some(open);
    }
    if c == '\\' && open == '"' {
        *escaped = true;
        return Some(open);
    }
    if c == open { None } else { Some(open) }
}

#[cfg(test)]
mod tests {
    use super::{parse, scan_flow, scan_quoted};
    use crate::parse_error_kind::ParseErrorKind;
    use crate::unsupported_syntax::UnsupportedSyntax;

    #[test]
    fn plain_drops_a_trailing_comment() {
        let value = parse("a: b c # note", 3_usize).expect("プレーンスカラー");
        assert_eq!(value.text(), "b c");
        assert_eq!(value.column().get(), 4_u32);
    }

    #[test]
    fn plain_keeps_a_hash_without_leading_space() {
        let value = parse("a: b#c", 3_usize).expect("プレーンスカラー");
        assert_eq!(value.text(), "b#c");
    }

    #[test]
    fn quoted_keeps_the_quotes_and_the_hash() {
        let value = parse("a: \"b # c\"", 3_usize).expect("クォートされたスカラー");
        assert_eq!(value.text(), "\"b # c\"");
    }

    #[test]
    fn unterminated_quote_is_unsupported() {
        let error = parse("a: \"b", 3_usize).expect_err("閉じないクォート");
        assert_eq!(
            error,
            ParseErrorKind::Unsupported(UnsupportedSyntax::MultiLineScalar)
        );
    }

    #[test]
    fn anchor_is_unsupported() {
        let error = parse("a: &x", 3_usize).expect_err("アンカー");
        assert_eq!(
            error,
            ParseErrorKind::Unsupported(UnsupportedSyntax::Anchor)
        );
    }

    #[test]
    fn scan_quoted_handles_single_quote_escape() {
        assert_eq!(scan_quoted("'a''b' rest", '\''), Some(6_usize));
    }

    #[test]
    fn scan_flow_ignores_brackets_inside_quotes() {
        assert_eq!(scan_flow("[a, \"]\", b] x"), Some(11_usize));
    }

    #[test]
    fn scan_flow_reports_an_open_bracket() {
        assert_eq!(scan_flow("[a, b"), None);
    }
}
