//! マッピングの1項目（`key: value`）の読み取り。

use alloc::borrow::ToOwned;
use alloc::string::String;

use crate::block_header;
use crate::entry_value::EntryValue;
use crate::key_span::KeySpan;
use crate::parse_error_kind::ParseErrorKind;
use crate::scalar_value::{self, scan_quoted};
use crate::unsupported_syntax::UnsupportedSyntax;

/// マッピングの1項目。
#[derive(Debug, Clone)]
pub(crate) struct MappingEntry {
    key: String,
    value: EntryValue,
    comment: Option<usize>,
}

impl MappingEntry {
    /// キー・値・行末コメントの位置（行頭からのバイト）から作る。
    pub(crate) fn new(key: String, value: EntryValue, comment: Option<usize>) -> Self {
        Self {
            key,
            value,
            comment,
        }
    }

    /// キー。クォートは外すが、中身のエスケープは解かない。
    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    /// この行の行末コメントの `#` の位置（行頭からのバイト）。
    ///
    /// 🔑 コメントは**値ではなく行**に属する。`key:` のように値が空でも、
    /// `key: |` のようにブロックの始まりでも、行末コメントは同じように書ける。
    pub(crate) fn comment(&self) -> Option<usize> {
        self.comment
    }

    /// 値を取り出す。
    pub(crate) fn into_value(self) -> EntryValue {
        self.value
    }
}

/// `line` の `start` バイト位置から `key: value` を読む。
///
/// キーの形をしていなければ `Ok(None)` を返す（呼び手が継続行として扱う）。
///
/// # Errors
///
/// 複合キー・マージキー、および値の側で読めない構文に出会ったらエラーにする。
pub(crate) fn parse(line: &str, start: usize) -> Result<Option<MappingEntry>, ParseErrorKind> {
    let rest = line.get(start..).unwrap_or("");
    if rest == "?" || rest.starts_with("? ") {
        return Err(ParseErrorKind::Unsupported(UnsupportedSyntax::ComplexKey));
    }
    if rest.starts_with("<<") {
        return Err(ParseErrorKind::Unsupported(UnsupportedSyntax::MergeKey));
    }
    let Some(key) = split_key(rest) else {
        return Ok(None);
    };
    let after_colon = key.colon().saturating_add(1_usize);
    let value_start = skip_spaces(rest, after_colon);
    // タグ（`!override` 等）は値ではないので、ここで読み飛ばす（v1.1）。
    // 後ろに何も残らなければ「空の値」になり、次の行の入れ子を受ける。
    let absolute = scalar_value::skip_tag(line, start.saturating_add(value_start));
    let value = read_value(line, absolute)?;
    let comment = comment_of(line, absolute, &value);
    Ok(Some(MappingEntry::new(key.into_text(), value, comment)))
}

/// `:` の右側を読む。
fn read_value(line: &str, absolute: usize) -> Result<EntryValue, ParseErrorKind> {
    let tail = line.get(absolute..).unwrap_or("");
    if tail.is_empty() || tail.starts_with('#') {
        return Ok(EntryValue::Empty);
    }
    if tail.starts_with('|') || tail.starts_with('>') {
        return block_header::parse(tail).map(EntryValue::Block);
    }
    scalar_value::parse(line, absolute).map(EntryValue::Scalar)
}

/// この項目の行末コメントの `#` の位置（行頭からのバイト）。
///
/// 値の種類ごとに、どこから先がコメントかを知っている場所が違う。
/// 🔑 ブロックの指示子（`|2-`）に使える文字は数字と `+` `-` だけなので、
/// [`block_header::parse`] が通った行に残る `#` は**必ずコメントの始まり**である。
fn comment_of(line: &str, absolute: usize, value: &EntryValue) -> Option<usize> {
    let tail = line.get(absolute..)?;
    match *value {
        EntryValue::Empty => tail.starts_with('#').then_some(absolute),
        EntryValue::Scalar(ref scalar) => scalar.comment(),
        EntryValue::Block(_) => tail.find('#').map(|at| absolute.saturating_add(at)),
    }
}

/// キーを読む。プレーンか、`"…"` / `'…'` のどちらか。
fn split_key(rest: &str) -> Option<KeySpan> {
    let first = rest.chars().next()?;
    if first == '"' || first == '\'' {
        return quoted_key(rest, first);
    }
    let colon = plain_key_end(rest)?;
    let text = rest.get(..colon).unwrap_or("").trim_end().to_owned();
    (!text.is_empty()).then(|| KeySpan::new(text, colon))
}

/// クォートされたキー。クォートは外すが、中身は原文のまま持つ。
fn quoted_key(rest: &str, quote: char) -> Option<KeySpan> {
    let end = scan_quoted(rest, quote)?;
    let inner = rest
        .get(1_usize..end.saturating_sub(1_usize))
        .unwrap_or("")
        .to_owned();
    let colon = skip_spaces(rest, end);
    rest.get(colon..)?
        .starts_with(':')
        .then(|| KeySpan::new(inner, colon))
}

/// プレーンキーを終わらせる `:` の位置。`:` の後ろは空白か行末でなければならない。
fn plain_key_end(rest: &str) -> Option<usize> {
    let mut previous: Option<char> = None;
    let mut chars = rest.char_indices().peekable();
    loop {
        let (index, c) = chars.next()?;
        if c == '#' && previous.is_none_or(char::is_whitespace) {
            return None;
        }
        if c == ':' && chars.peek().is_none_or(|&(_, next)| next == ' ') {
            return Some(index);
        }
        previous = Some(c);
    }
}

/// `from` から続く空白を読み飛ばした位置。
fn skip_spaces(text: &str, from: usize) -> usize {
    let tail = text.get(from..).unwrap_or("");
    let spaces = tail
        .len()
        .saturating_sub(tail.trim_start_matches(' ').len());
    from.saturating_add(spaces)
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::entry_value::EntryValue;
    use crate::parse_error_kind::ParseErrorKind;
    use crate::unsupported_syntax::UnsupportedSyntax;

    #[test]
    fn reads_a_plain_key_and_value() {
        let entry = parse("  run: npm ci", 2_usize)
            .expect("読めるはず")
            .expect("項目である");
        assert_eq!(entry.key(), "run");
        match entry.into_value() {
            EntryValue::Scalar(value) => assert_eq!(value.text(), "npm ci"),
            EntryValue::Empty | EntryValue::Block(_) => panic!("スカラーのはず"),
        }
    }

    #[test]
    fn reads_a_quoted_key() {
        let entry = parse("\"weird key\": 1", 0_usize)
            .expect("読めるはず")
            .expect("項目である");
        assert_eq!(entry.key(), "weird key");
    }

    #[test]
    fn an_empty_value_is_not_a_scalar() {
        let entry = parse("on:", 0_usize)
            .expect("読めるはず")
            .expect("項目である");
        assert_eq!(entry.comment(), None);
        assert!(matches!(entry.into_value(), EntryValue::Empty));
    }

    /// 行末コメントは、値が空でもブロックの始まりでも同じように見つかる。
    #[test]
    fn a_trailing_comment_is_located_for_every_kind_of_value() {
        let scalar = parse("run: npm ci # note", 0_usize)
            .expect("読めるはず")
            .expect("項目である");
        assert_eq!(scalar.comment(), Some(12_usize));

        let empty = parse("on: # note", 0_usize)
            .expect("読めるはず")
            .expect("項目である");
        assert_eq!(empty.comment(), Some(4_usize));

        let block = parse("run: |2- # note", 0_usize)
            .expect("読めるはず")
            .expect("項目である");
        assert_eq!(block.comment(), Some(9_usize));
    }

    #[test]
    fn a_line_without_a_key_is_not_an_entry() {
        assert!(parse("just words", 0_usize).expect("読めるはず").is_none());
    }

    #[test]
    fn a_colon_without_a_space_is_not_a_key() {
        assert!(
            parse("http://example.test", 0_usize)
                .expect("読めるはず")
                .is_none()
        );
    }

    #[test]
    fn a_merge_key_is_unsupported() {
        assert_eq!(
            parse("<<: *base", 0_usize).expect_err("マージキー"),
            ParseErrorKind::Unsupported(UnsupportedSyntax::MergeKey)
        );
    }
}
