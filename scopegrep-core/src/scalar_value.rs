//! 1行スカラーの読み取り。

use alloc::borrow::ToOwned;
use alloc::string::String;

use crate::column::Column;
use crate::flow_scan::FlowScan;
use crate::flow_state::FlowState;
use crate::malformed_input::MalformedInput;
use crate::parse_error_kind::ParseErrorKind;
use crate::unsupported_syntax::UnsupportedSyntax;

/// 読み取ったスカラー値と、その先頭の桁と、後ろに続く行末コメントの位置。
///
/// 値は**原文のまま**持つ（クォートの中身をエスケープ解除しない）。
/// 人が `grep` で見る文字列と同じものに当たることを優先する（設計メモ「検索の意味」）。
///
/// 🔑 行末コメントの位置を**ここで返す**。値の終わりを知っているのは値を読んだ側だけで、
/// 後から行を見直して `#` を探すと、`run: echo "a # b"` のような
/// 「プレーンスカラーの中のクォートらしきもの」で読み方が食い違う。
#[derive(Debug, Clone)]
pub(crate) struct ScalarValue {
    text: String,
    column: Column,
    comment: Option<usize>,
    flow: FlowState,
}

impl ScalarValue {
    /// 値・桁・行末コメントの位置（行頭からのバイト）から作る。
    pub(crate) fn new(text: String, column: Column, comment: Option<usize>) -> Self {
        Self {
            text,
            column,
            comment,
            flow: FlowState::Complete,
        }
    }

    /// 閉じていないフロー記法の**最初の行**を作る。
    ///
    /// 行末までが値であり、行末コメントは無い（`#` は括弧の中なので値の一部である）。
    pub(crate) fn opening(text: String, column: Column, scan: FlowScan) -> Self {
        Self {
            text,
            column,
            comment: None,
            flow: FlowState::Unclosed(scan),
        }
    }

    /// この行でフロー記法が閉じたか、次の行へ続くか。
    pub(crate) fn flow(&self) -> FlowState {
        self.flow
    }

    /// 原文のままの値。
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// 値の先頭の桁。
    pub(crate) fn column(&self) -> Column {
        self.column
    }

    /// この値の後ろに続く行末コメントの `#` の位置（行頭からのバイト）。
    pub(crate) fn comment(&self) -> Option<usize> {
        self.comment
    }

    /// 値を取り出す。
    pub(crate) fn into_text(self) -> String {
        self.text
    }
}

/// `line` の `start` バイト位置から始まる1行スカラーを読む。
///
/// フロー記法が行内で閉じないときは、**その行までを値として返す**
/// （[`ScalarValue::flow`] が続きを持ち越す。v1.1）。
///
/// # Errors
///
/// アンカー・エイリアス・行内で閉じないクォートはエラーにする。
pub(crate) fn parse(line: &str, start: usize) -> Result<ScalarValue, ParseErrorKind> {
    let rest = line.get(start..).unwrap_or("");
    let column = Column::after(line.get(..start).unwrap_or("").chars().count());
    let Some(first) = rest.chars().next() else {
        return Ok(ScalarValue::new(String::new(), column, None));
    };
    let end = match first {
        '&' => return Err(ParseErrorKind::Unsupported(UnsupportedSyntax::Anchor)),
        '*' => return Err(ParseErrorKind::Unsupported(UnsupportedSyntax::Alias)),
        '"' | '\'' => closing(scan_quoted(rest, first), UnsupportedSyntax::MultiLineScalar)?,
        '[' | '{' => {
            let mut scan = FlowScan::start();
            let Some(end) = scan.advance(rest) else {
                // 🔴 閉じていない間は**行末までが値**である。括弧の中の `#` は
                // コメントではない（設計メモ「フロー記法」・v1.1 の割り切り）。
                // 行末コメントとして扱うのは、閉じ括弧より**後ろ**の `#` だけである。
                return Ok(ScalarValue::opening(rest.to_owned(), column, scan));
            };
            end
        }
        _ => return Ok(plain(rest, column, start)),
    };
    bounded(rest, end, column, start)
}

/// 閉じ位置を取り出す。閉じていなければ `missing` を返す。
fn closing(end: Option<usize>, missing: UnsupportedSyntax) -> Result<usize, ParseErrorKind> {
    end.ok_or(ParseErrorKind::Unsupported(missing))
}

/// 値の先頭にタグ（`!override` / `!!str`）があれば、その後ろの位置を返す。
///
/// 🔑 タグは**値ではない**ので、読み飛ばして検索に当てない（設計メモ・v1.1）。
/// 実ファイル計測では compose の override が 3 件これで落ちていた。
/// 後ろに何も残らなければ「空の値」と同じで、次の行の入れ子を受ける。
pub(crate) fn skip_tag(line: &str, at: usize) -> usize {
    let rest = line.get(at..).unwrap_or("");
    if !rest.starts_with('!') {
        return at;
    }
    let width = rest.find(' ').unwrap_or(rest.len());
    let after = at.saturating_add(width);
    let tail = line.get(after..).unwrap_or("");
    after.saturating_add(
        tail.len()
            .saturating_sub(tail.trim_start_matches(' ').len()),
    )
}

/// 閉じ位置が分かっている値を切り出す。**後ろに許すのは空白と行末コメントだけ**である。
pub(crate) fn bounded(
    rest: &str,
    end: usize,
    column: Column,
    start: usize,
) -> Result<ScalarValue, ParseErrorKind> {
    let after = rest.get(end..).unwrap_or("");
    let tail = after.trim();
    if !tail.is_empty() && !tail.starts_with('#') {
        return Err(ParseErrorKind::Malformed(MalformedInput::TrailingContent));
    }
    let spaces = after.len().saturating_sub(after.trim_start().len());
    let at = start.saturating_add(end).saturating_add(spaces);
    Ok(ScalarValue::new(
        rest.get(..end).unwrap_or("").to_owned(),
        column,
        tail.starts_with('#').then_some(at),
    ))
}

/// プレーンスカラー。**空白の直後の `#` から行末まで**はコメントなので落とす。
fn plain(rest: &str, column: Column, start: usize) -> ScalarValue {
    let cut = comment_start(rest);
    let text = rest
        .get(..cut.unwrap_or(rest.len()))
        .unwrap_or("")
        .trim_end()
        .to_owned();
    ScalarValue::new(text, column, cut.map(|at| start.saturating_add(at)))
}

/// プレーンスカラーを終わらせる `#` の位置（`rest` からの相対）。
///
/// 🔴 空白の直後の `#` だけがコメントである。`b#c` の `#` は値の一部であり、
/// ここを緩めると URL やフラグメントを含む値が切れる。
fn comment_start(rest: &str) -> Option<usize> {
    let mut previous: Option<char> = None;
    for (index, c) in rest.char_indices() {
        if c == '#' && previous.is_none_or(char::is_whitespace) {
            return Some(index);
        }
        previous = Some(c);
    }
    None
}

/// クォートを**1枚だけ**外す。中身のエスケープは解かない。
///
/// キーの扱い（`mapping_entry`）と同じ規則をラベルにも当てる。
/// `"Build"` のラベルは `Build`、`'A ''b'''` のラベルは `A ''b''` である。
/// 🔑 ラベルは**表示のための識別子**であって検索対象ではないので、
/// 値の「原文のまま」規則（設計メモ「検索の意味」）は当てはめない。
pub(crate) fn unquote(text: &str) -> &str {
    let mut chars = text.chars();
    let (Some(first), Some(last)) = (chars.next(), chars.next_back()) else {
        return text;
    };
    if first == last && (first == '"' || first == '\'') {
        return chars.as_str();
    }
    text
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

#[cfg(test)]
mod tests {
    use super::{parse, scan_quoted, skip_tag, unquote};
    use crate::flow_state::FlowState;

    #[test]
    fn unquote_strips_one_layer_of_double_quotes() {
        assert_eq!(unquote("\"Build\""), "Build");
    }

    /// 🔴 1枚だけ外す。中の `''` エスケープは解かない。
    #[test]
    fn unquote_strips_one_layer_of_single_quotes() {
        assert_eq!(unquote("'A ''b'''"), "A ''b''");
    }

    #[test]
    fn unquote_leaves_a_plain_scalar_alone() {
        assert_eq!(unquote("Build"), "Build");
        assert_eq!(unquote("\""), "\"");
        assert_eq!(unquote(""), "");
    }
    use crate::parse_error_kind::ParseErrorKind;
    use crate::unsupported_syntax::UnsupportedSyntax;

    #[test]
    fn plain_drops_a_trailing_comment() {
        let value = parse("a: b c # note", 3_usize).expect("プレーンスカラー");
        assert_eq!(value.text(), "b c");
        assert_eq!(value.column().get(), 4_u32);
        // 値からは落とすが、位置は捨てない（`--comments` はここから出る）。
        assert_eq!(value.comment(), Some(7_usize));
    }

    #[test]
    fn plain_keeps_a_hash_without_leading_space() {
        let value = parse("a: b#c", 3_usize).expect("プレーンスカラー");
        assert_eq!(value.text(), "b#c");
        assert_eq!(value.comment(), None);
    }

    #[test]
    fn quoted_keeps_the_quotes_and_the_hash() {
        let value = parse("a: \"b # c\"", 3_usize).expect("クォートされたスカラー");
        assert_eq!(value.text(), "\"b # c\"");
        assert_eq!(value.comment(), None);
    }

    /// クォートやフロー記法の**後ろ**に来た `#` は行末コメントである。
    #[test]
    fn a_comment_after_a_closed_value_is_located() {
        let quoted = parse("a: \"b\"  # note", 3_usize).expect("クォートされたスカラー");
        assert_eq!(quoted.text(), "\"b\"");
        assert_eq!(quoted.comment(), Some(8_usize));

        let flow = parse("a: [x] # note", 3_usize).expect("フロー記法");
        assert_eq!(flow.text(), "[x]");
        assert_eq!(flow.comment(), Some(7_usize));
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

    /// 行内で閉じたフローは、その場で完結する。
    #[test]
    fn a_closed_flow_is_complete() {
        let value = parse("a: [x, y] # note", 3_usize).expect("フロー記法");
        assert_eq!(value.text(), "[x, y]");
        assert_eq!(value.comment(), Some(10_usize));
        assert!(matches!(value.flow(), FlowState::Complete));
    }

    /// 🔴 閉じないフローは**行末までが値**になり、続きを持ち越す。
    /// 括弧の中の `#` は行末コメントではない（v1.1 の割り切り）。
    #[test]
    fn an_open_flow_keeps_the_rest_of_the_line_as_its_value() {
        let value = parse("a: [x, # y", 3_usize).expect("フロー記法");
        assert_eq!(value.text(), "[x, # y");
        assert_eq!(value.comment(), None);
        assert!(matches!(value.flow(), FlowState::Unclosed(_)));
    }

    #[test]
    fn skip_tag_moves_past_the_tag_and_its_spaces() {
        assert_eq!(skip_tag("a: !!str 123", 3_usize), 9_usize);
        assert_eq!(skip_tag("a: !override", 3_usize), 12_usize);
        assert_eq!(skip_tag("a: plain", 3_usize), 3_usize);
    }

    /// タグを読み飛ばした後ろは、普通の値として読める。
    #[test]
    fn a_value_after_a_tag_is_read_normally() {
        let at = skip_tag("a: !reset []", 3_usize);
        let value = parse("a: !reset []", at).expect("タグの後ろの値");
        assert_eq!(value.text(), "[]");
    }
}
