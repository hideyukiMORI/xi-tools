//! JSON（RFC 8259）を読む。
//!
//! 🔴 **`serde_json` を入れない**（ADR 0003）。GraphQL の応答を読むために 5 crate を
//! 引き込むと、中核が `alloc` だけで閉じなくなる。JSON は仕様が小さく、fixture で
//! 全部試験できる大きさなので、ここに手で書く。
//!
//! 🔑 **どの入力でも panic しない**。添字を `get` に置き換えるだけでなく、入れ子には
//! 上限（`MAX_DEPTH` = 128）を置いてスタックを溢れさせない。応答は向こう側が作るもので、
//! こちらの前提は当てにならない。

use alloc::string::String;
use alloc::vec::Vec;

use crate::json_error::JsonError;
use crate::json_error_kind::JsonErrorKind;
use crate::json_number::JsonNumber;
use crate::json_value::JsonValue;

/// 配列とオブジェクトの入れ子の上限。
///
/// 🔑 再帰下降で読むので、深さがそのままスタックの深さになる。GitHub の応答は
/// 10 段も無いが、**壊れた入力で落ちない**ことのほうが大事なので上限で止める。
const MAX_DEPTH: usize = 128;

/// UTF-16 の上位サロゲートの下限。
const HIGH_SURROGATE_START: u32 = 0xD800;
/// UTF-16 の下位サロゲートの下限。
const LOW_SURROGATE_START: u32 = 0xDC00;
/// UTF-16 の下位サロゲートの上限。
const LOW_SURROGATE_END: u32 = 0xDFFF;

/// JSON を読む。
///
/// 上位の値 1 つと、その前後の空白だけを受け付ける。空白は ` ` `\t` `\n` `\r`
/// の 4 つ（RFC 8259 §2）で、それ以外の空白文字は空白ではない。
///
/// # Errors
///
/// 文法から外れたら [`JsonError`] を返す。エラーは**必ず位置（文字数）と種別**を持つ。
pub fn parse_json(source: &str) -> Result<JsonValue, JsonError> {
    let mut parser = Parser::new(source);
    parser.skip_whitespace();
    let value = parser.value(0_usize)?;
    parser.skip_whitespace();
    if parser.peek().is_some() {
        return Err(parser.error(JsonErrorKind::TrailingCharacters));
    }
    Ok(value)
}

/// 読み進める位置。残りの入力と、そこまでに読んだ**文字数**を持つ。
#[derive(Debug)]
struct Parser<'a> {
    rest: &'a str,
    offset: usize,
}

impl<'a> Parser<'a> {
    /// 入力の先頭から始める。
    fn new(source: &'a str) -> Self {
        Self {
            rest: source,
            offset: 0_usize,
        }
    }

    /// 次の 1 文字を見る。進めない。
    fn peek(&self) -> Option<char> {
        self.rest.chars().next()
    }

    /// `bytes` バイト・`chars` 文字ぶん進む。
    fn advance(&mut self, bytes: usize, chars: usize) {
        self.rest = self.rest.get(bytes..).unwrap_or("");
        self.offset = self.offset.saturating_add(chars);
    }

    /// 次の 1 文字を取り出して進む。
    fn next_char(&mut self) -> Option<char> {
        let character = self.rest.chars().next()?;
        self.advance(character.len_utf8(), 1_usize);
        Some(character)
    }

    /// 先読みで確認済みの 1 文字を捨てる。
    fn discard(&mut self) {
        if let Some(character) = self.rest.chars().next() {
            self.advance(character.len_utf8(), 1_usize);
        }
    }

    /// `word` が先頭にあれば読み飛ばす。
    fn consume(&mut self, word: &str) -> bool {
        let Some(rest) = self.rest.strip_prefix(word) else {
            return false;
        };
        self.offset = self.offset.saturating_add(word.chars().count());
        self.rest = rest;
        true
    }

    /// RFC 8259 の空白（` ` `\t` `\n` `\r`）を読み飛ばす。
    fn skip_whitespace(&mut self) {
        while let Some(character) = self.peek() {
            if !matches!(character, ' ' | '\t' | '\n' | '\r') {
                break;
            }
            self.discard();
        }
    }

    /// 今の位置でエラーを作る。
    fn error(&self, kind: JsonErrorKind) -> JsonError {
        JsonError::new(self.offset, kind)
    }

    /// 今の位置の文字が文法に合わないときのエラー。入力が尽きていれば `UnexpectedEnd`。
    fn unexpected(&self) -> JsonError {
        match self.peek() {
            Some(character) => self.error(JsonErrorKind::UnexpectedCharacter(character)),
            None => self.error(JsonErrorKind::UnexpectedEnd),
        }
    }

    /// 1 段深く入る。上限を超えたら `TooDeep`。
    fn enter(&self, depth: usize) -> Result<usize, JsonError> {
        let inner = depth.saturating_add(1_usize);
        if inner > MAX_DEPTH {
            return Err(self.error(JsonErrorKind::TooDeep));
        }
        Ok(inner)
    }

    /// 値を 1 つ読む。`depth` は今いる入れ子の深さ（上位の値は 0）。
    fn value(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        let Some(character) = self.peek() else {
            return Err(self.error(JsonErrorKind::UnexpectedEnd));
        };
        match character {
            'n' => self.keyword("null", JsonValue::Null),
            't' => self.keyword("true", JsonValue::Bool(true)),
            'f' => self.keyword("false", JsonValue::Bool(false)),
            '"' => self.string().map(JsonValue::String),
            '[' => self.array(depth),
            '{' => self.object(depth),
            '-' | '.' | '0'..='9' => self.number(),
            other => Err(self.error(JsonErrorKind::UnexpectedCharacter(other))),
        }
    }

    /// `null` / `true` / `false` を読む。
    fn keyword(&mut self, word: &str, value: JsonValue) -> Result<JsonValue, JsonError> {
        if self.consume(word) {
            return Ok(value);
        }
        Err(self.unexpected())
    }

    /// 配列を読む。呼ばれた時点で先頭は `[`。
    fn array(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        let inner = self.enter(depth)?;
        self.discard();
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.peek() == Some(']') {
            self.discard();
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.value(inner)?);
            if !self.separator(']')? {
                return Ok(JsonValue::Array(items));
            }
        }
    }

    /// オブジェクトを読む。呼ばれた時点で先頭は `{`。
    fn object(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        let inner = self.enter(depth)?;
        self.discard();
        self.skip_whitespace();
        let mut members = Vec::new();
        if self.peek() == Some('}') {
            self.discard();
            return Ok(JsonValue::Object(members));
        }
        loop {
            members.push(self.member(inner)?);
            if !self.separator('}')? {
                return Ok(JsonValue::Object(members));
            }
        }
    }

    /// オブジェクトの `"キー": 値` を 1 組読む。
    fn member(&mut self, depth: usize) -> Result<(String, JsonValue), JsonError> {
        self.skip_whitespace();
        if self.peek() != Some('"') {
            return Err(self.unexpected());
        }
        let key = self.string()?;
        self.skip_whitespace();
        if self.peek() != Some(':') {
            return Err(self.unexpected());
        }
        self.discard();
        self.skip_whitespace();
        let value = self.value(depth)?;
        Ok((key, value))
    }

    /// 要素の後を読む。`,` なら `true`（次の要素へ）、`close` なら `false`（閉じた）。
    fn separator(&mut self, close: char) -> Result<bool, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            Some(',') => {
                self.discard();
                self.skip_whitespace();
                Ok(true)
            }
            Some(character) if character == close => {
                self.discard();
                Ok(false)
            }
            Some(character) => Err(self.error(JsonErrorKind::UnexpectedCharacter(character))),
            None => Err(self.error(JsonErrorKind::UnexpectedEnd)),
        }
    }

    /// 文字列を読む。呼ばれた時点で先頭は `"`。
    fn string(&mut self) -> Result<String, JsonError> {
        self.discard();
        let mut text = String::new();
        loop {
            let at = self.offset;
            let Some(character) = self.next_char() else {
                return Err(self.error(JsonErrorKind::UnexpectedEnd));
            };
            match character {
                '"' => return Ok(text),
                '\\' => text.push(self.escape()?),
                control if control < ' ' => {
                    return Err(JsonError::new(at, JsonErrorKind::ControlCharacterInString));
                }
                other => text.push(other),
            }
        }
    }

    /// `\` の後を読む。返るのは**1 文字**（サロゲートペアは結合した後）。
    fn escape(&mut self) -> Result<char, JsonError> {
        let Some(character) = self.next_char() else {
            return Err(self.error(JsonErrorKind::UnexpectedEnd));
        };
        match character {
            '"' => Ok('"'),
            '\\' => Ok('\\'),
            '/' => Ok('/'),
            'b' => Ok('\u{8}'),
            'f' => Ok('\u{c}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'u' => self.unicode_escape(),
            _ => Err(self.error(JsonErrorKind::InvalidEscape)),
        }
    }

    /// `\uXXXX` を読む。上位サロゲートなら、続く `\uXXXX` と結合する。
    fn unicode_escape(&mut self) -> Result<char, JsonError> {
        let first = self.hex4()?;
        if !is_high_surrogate(first) {
            return char::from_u32(first)
                .ok_or_else(|| self.error(JsonErrorKind::InvalidUnicodeEscape));
        }
        if !self.consume("\\u") {
            return Err(self.error(JsonErrorKind::InvalidUnicodeEscape));
        }
        let second = self.hex4()?;
        combine_surrogates(first, second)
            .ok_or_else(|| self.error(JsonErrorKind::InvalidUnicodeEscape))
    }

    /// 16 進 4 桁を読む。
    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut value = 0_u32;
        for _ in 0_u8..4_u8 {
            let Some(character) = self.next_char() else {
                return Err(self.error(JsonErrorKind::UnexpectedEnd));
            };
            let Some(digit) = character.to_digit(16_u32) else {
                return Err(self.error(JsonErrorKind::InvalidUnicodeEscape));
            };
            value = value.saturating_mul(16_u32).saturating_add(digit);
        }
        Ok(value)
    }

    /// 数を読む。文法から外れたら `InvalidNumber`。
    fn number(&mut self) -> Result<JsonValue, JsonError> {
        let invalid = self.error(JsonErrorKind::InvalidNumber);
        let Some(length) = number_length(self.rest) else {
            return Err(invalid);
        };
        let Some(lexeme) = self.rest.get(..length) else {
            return Err(invalid);
        };
        if continues_number(self.rest.get(length..).unwrap_or("")) {
            return Err(invalid);
        }
        let number = JsonNumber::new(String::from(lexeme));
        self.advance(length, lexeme.chars().count());
        Ok(JsonValue::Number(number))
    }
}

/// 上位サロゲートか。
fn is_high_surrogate(value: u32) -> bool {
    (HIGH_SURROGATE_START..LOW_SURROGATE_START).contains(&value)
}

/// サロゲートペアを 1 文字に結合する。下位が範囲外なら `None`（孤立サロゲート）。
fn combine_surrogates(high: u32, low: u32) -> Option<char> {
    if !(LOW_SURROGATE_START..=LOW_SURROGATE_END).contains(&low) {
        return None;
    }
    let upper = high
        .checked_sub(HIGH_SURROGATE_START)?
        .checked_mul(0x400_u32)?;
    let lower = low.checked_sub(LOW_SURROGATE_START)?;
    let scalar = 0x1_0000_u32.checked_add(upper)?.checked_add(lower)?;
    char::from_u32(scalar)
}

/// RFC 8259 の number の文法に合う先頭部分の**バイト長**。合わなければ `None`。
fn number_length(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let start = usize::from(bytes.first() == Some(&b'-'));
    let after_integer = integer_length(bytes, start)?;
    let after_fraction = fraction_length(bytes, after_integer)?;
    exponent_length(bytes, after_fraction)
}

/// 整数部の終わり。`0` の後に数字は続けられない（先頭 0 の禁止）。
fn integer_length(bytes: &[u8], from: usize) -> Option<usize> {
    match bytes.get(from) {
        Some(&b'0') => Some(from.saturating_add(1_usize)),
        Some(&digit) if digit.is_ascii_digit() => Some(digits_end(bytes, from)),
        Some(_) | None => None,
    }
}

/// 小数部の終わり。`.` が無ければそのまま。`.` の後に数字が無ければ `None`。
fn fraction_length(bytes: &[u8], from: usize) -> Option<usize> {
    if bytes.get(from) != Some(&b'.') {
        return Some(from);
    }
    let start = from.saturating_add(1_usize);
    let end = digits_end(bytes, start);
    (end > start).then_some(end)
}

/// 指数部の終わり。`e` / `E` が無ければそのまま。数字が無ければ `None`。
fn exponent_length(bytes: &[u8], from: usize) -> Option<usize> {
    let marker = bytes.get(from).copied();
    if marker != Some(b'e') && marker != Some(b'E') {
        return Some(from);
    }
    let after_marker = from.saturating_add(1_usize);
    let sign = bytes.get(after_marker).copied();
    let start = if sign == Some(b'+') || sign == Some(b'-') {
        after_marker.saturating_add(1_usize)
    } else {
        after_marker
    };
    let end = digits_end(bytes, start);
    (end > start).then_some(end)
}

/// 数字が続く限り進んだ位置。
fn digits_end(bytes: &[u8], from: usize) -> usize {
    let mut index = from;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index = index.saturating_add(1_usize);
    }
    index
}

/// 数の直後に数の続きらしきものが残っているか。
///
/// 🔑 `01` は「`0` を読んで `1` が余った」ではなく**数の書き方の誤り**である。
/// ここで見ないと `TrailingCharacters` になり、人が原因を取り違える。
fn continues_number(rest: &str) -> bool {
    rest.as_bytes()
        .first()
        .copied()
        .is_some_and(|byte| byte.is_ascii_digit() || byte == b'.' || byte == b'e' || byte == b'E')
}

#[cfg(test)]
mod tests {
    use super::parse_json;
    use crate::json_error_kind::JsonErrorKind;
    use crate::json_number::JsonNumber;
    use crate::json_value::JsonValue;
    use alloc::format;
    use alloc::string::String;

    /// GitHub GraphQL の実応答と同じ**構造**を持つ架空の fixture。
    /// 🔴 実データからコピーしない（顧客と金の情報を public リポに持ち込まないため）。
    const GRAPHQL_RESPONSE: &str = include_str!("../testdata/graphql-response.json");

    fn kind(source: &str) -> JsonErrorKind {
        *parse_json(source).expect_err("読めない入力である").kind()
    }

    fn offset(source: &str) -> usize {
        parse_json(source).expect_err("読めない入力である").offset()
    }

    fn text_of(source: &str) -> String {
        let value = parse_json(source).expect("読める入力である");
        String::from(value.as_str().expect("文字列である"))
    }

    // ── 値の 6 種 ──────────────────────────────────────────────────────────

    #[test]
    fn reads_the_six_kinds_of_value() {
        assert_eq!(parse_json("null"), Ok(JsonValue::Null));
        assert_eq!(parse_json("true"), Ok(JsonValue::Bool(true)));
        assert_eq!(parse_json("false"), Ok(JsonValue::Bool(false)));
        assert_eq!(
            parse_json("\"main\""),
            Ok(JsonValue::String(String::from("main")))
        );
        assert_eq!(
            parse_json("12"),
            Ok(JsonValue::Number(JsonNumber::new(String::from("12"))))
        );
        assert_eq!(
            parse_json("[]")
                .expect("配列である")
                .as_array()
                .map(<[JsonValue]>::is_empty),
            Some(true)
        );
        assert_eq!(
            parse_json("{}")
                .expect("オブジェクトである")
                .as_object()
                .map(<[(String, JsonValue)]>::is_empty),
            Some(true)
        );
    }

    #[test]
    fn reads_nested_structures() {
        let value = parse_json(r#"{"a":[1,{"b":null}],"c":{}}"#).expect("読める");
        let inner = value
            .get("a")
            .and_then(JsonValue::as_array)
            .and_then(|items| items.get(1_usize))
            .expect("2 番目の要素がある");
        assert!(inner.get("b").is_some_and(JsonValue::is_null));
        assert_eq!(
            value
                .get("c")
                .and_then(JsonValue::as_object)
                .map(<[(String, JsonValue)]>::is_empty),
            Some(true)
        );
    }

    // ── 文字列 ─────────────────────────────────────────────────────────────

    #[test]
    fn reads_every_escape() {
        assert_eq!(
            text_of(r#""\"\\\/\b\f\n\r\t""#),
            String::from("\"\\/\u{8}\u{c}\n\r\t")
        );
        assert_eq!(text_of(r#""あ""#), String::from("あ"));
    }

    /// サロゲートペアは 1 文字に結合する。
    #[test]
    fn joins_a_surrogate_pair() {
        assert_eq!(text_of(r#""\uD83D\uDE00""#), String::from("😀"));
        assert_eq!(text_of(r#""\ud83d\ude00""#), String::from("😀"));
        assert_eq!(text_of(r#""\u3042""#), String::from("あ"));
        assert_eq!(text_of(r#""\u0041""#), String::from("A"));
    }

    #[test]
    fn refuses_a_lone_surrogate() {
        assert_eq!(kind(r#""\uD83D""#), JsonErrorKind::InvalidUnicodeEscape);
        assert_eq!(kind(r#""\uDE00""#), JsonErrorKind::InvalidUnicodeEscape);
        assert_eq!(kind(r#""\uD83Dx""#), JsonErrorKind::InvalidUnicodeEscape);
        assert_eq!(kind(r#""\uD83DA""#), JsonErrorKind::InvalidUnicodeEscape);
        assert_eq!(kind(r#""\u00ZZ""#), JsonErrorKind::InvalidUnicodeEscape);
        // 上位サロゲートの後に、また上位サロゲートが来る。
        assert_eq!(
            kind(r#""\uD83D\uD83D""#),
            JsonErrorKind::InvalidUnicodeEscape
        );
    }

    #[test]
    fn refuses_an_unknown_escape() {
        assert_eq!(kind(r#""\q""#), JsonErrorKind::InvalidEscape);
    }

    #[test]
    fn refuses_a_raw_control_character() {
        assert_eq!(kind("\"a\nb\""), JsonErrorKind::ControlCharacterInString);
        assert_eq!(kind("\"\u{0}\""), JsonErrorKind::ControlCharacterInString);
    }

    /// 制御文字以外の非 ASCII はそのまま通す。
    #[test]
    fn passes_non_ascii_through() {
        assert_eq!(text_of("\"日本語 😀\""), String::from("日本語 😀"));
    }

    #[test]
    fn refuses_an_unterminated_string() {
        assert_eq!(kind("\"abc"), JsonErrorKind::UnexpectedEnd);
        assert_eq!(kind(r#""\"#), JsonErrorKind::UnexpectedEnd);
        assert_eq!(kind(r#""\u12"#), JsonErrorKind::UnexpectedEnd);
    }

    // ── 数 ─────────────────────────────────────────────────────────────────

    #[test]
    fn accepts_the_number_grammar() {
        for source in ["-0", "0", "1e5", "1.5E-3", "-1.5e+10", "1234567890"] {
            assert!(parse_json(source).is_ok(), "{source} は読めるはずである");
        }
    }

    #[test]
    fn refuses_malformed_numbers() {
        for source in ["01", ".5", "1.", "1e", "-", "-.5", "1e+", "00"] {
            assert_eq!(
                kind(source),
                JsonErrorKind::InvalidNumber,
                "{source} は数として拒むはずである"
            );
        }
    }

    #[test]
    fn keeps_the_number_lexeme() {
        let value = parse_json("  -0.500e+2  ").expect("読める");
        assert_eq!(value.as_number().map(JsonNumber::lexeme), Some("-0.500e+2"));
    }

    // ── 空白と余分な文字 ───────────────────────────────────────────────────

    #[test]
    fn allows_whitespace_around_and_between_tokens() {
        let value = parse_json(" \t\r\n{ \"a\" : [ 1 , 2 ] } \n").expect("読める");
        assert_eq!(
            value
                .get("a")
                .and_then(JsonValue::as_array)
                .map(<[JsonValue]>::len),
            Some(2_usize)
        );
    }

    /// RFC 8259 の空白は 4 つだけ。全角空白は空白ではない。
    #[test]
    fn refuses_other_whitespace() {
        assert_eq!(
            kind("\u{3000}1"),
            JsonErrorKind::UnexpectedCharacter('\u{3000}')
        );
    }

    #[test]
    fn refuses_trailing_characters() {
        assert_eq!(kind("1 2"), JsonErrorKind::TrailingCharacters);
        assert_eq!(kind("{} x"), JsonErrorKind::TrailingCharacters);
        assert_eq!(kind("[1],"), JsonErrorKind::TrailingCharacters);
    }

    #[test]
    fn refuses_an_empty_input() {
        assert_eq!(kind(""), JsonErrorKind::UnexpectedEnd);
        assert_eq!(kind("   "), JsonErrorKind::UnexpectedEnd);
    }

    #[test]
    fn refuses_a_broken_structure() {
        let refused = [
            ("[1 2]", JsonErrorKind::UnexpectedCharacter('2')),
            ("[1,", JsonErrorKind::UnexpectedEnd),
            ("[1", JsonErrorKind::UnexpectedEnd),
            ("{", JsonErrorKind::UnexpectedEnd),
            (r#"{"a":1"#, JsonErrorKind::UnexpectedEnd),
            ("{1:2}", JsonErrorKind::UnexpectedCharacter('1')),
            (r#"{"a" 1}"#, JsonErrorKind::UnexpectedCharacter('1')),
            ("nul", JsonErrorKind::UnexpectedCharacter('n')),
            ("tru", JsonErrorKind::UnexpectedCharacter('t')),
            ("fals", JsonErrorKind::UnexpectedCharacter('f')),
            ("x", JsonErrorKind::UnexpectedCharacter('x')),
        ];
        for (source, expected) in refused {
            assert_eq!(kind(source), expected, "{source} の読み方が違う");
        }
    }

    // ── 入れ子の上限 ───────────────────────────────────────────────────────

    /// 🔴 129 段の `[` で **panic せず** `TooDeep` を返す。
    #[test]
    fn stops_before_the_stack_runs_out() {
        let mut deep = String::new();
        for _ in 0_u32..129_u32 {
            deep.push('[');
        }
        assert_eq!(kind(&deep), JsonErrorKind::TooDeep);
    }

    /// 上限ちょうど（128 段）は読める。
    #[test]
    fn accepts_the_deepest_allowed_nesting() {
        let mut deep = String::new();
        for _ in 0_u32..128_u32 {
            deep.push('[');
        }
        for _ in 0_u32..128_u32 {
            deep.push(']');
        }
        assert!(parse_json(&deep).is_ok());
    }

    // ── 重複キー ───────────────────────────────────────────────────────────

    #[test]
    fn duplicate_keys_keep_both_and_get_returns_the_last() {
        let value = parse_json(r#"{"state":"PENDING","state":"SUCCESS"}"#).expect("読める");
        assert_eq!(
            value.as_object().map(<[(String, JsonValue)]>::len),
            Some(2_usize)
        );
        assert_eq!(
            value.get("state").and_then(JsonValue::as_str),
            Some("SUCCESS")
        );
    }

    // ── 位置 ───────────────────────────────────────────────────────────────

    /// 🔴 位置は**バイト数ではなく文字数**である。日本語で確かめる。
    #[test]
    fn the_offset_counts_characters_not_bytes() {
        // `{"あいう": x}` の `x` は 8 文字目（0 起点）だが、バイトで数えると 14 である。
        let source = "{\"あいう\": x}";
        assert_eq!(kind(source), JsonErrorKind::UnexpectedCharacter('x'));
        assert_eq!(offset(source), 8_usize);
        assert_eq!(source.find('x'), Some(14_usize));
    }

    #[test]
    fn the_display_message_carries_the_offset_and_the_kind() {
        let error = parse_json("[1 2]").expect_err("読めない");
        let message = format!("{error}");
        assert!(message.contains('3'), "{message} に位置が無い");
        assert!(message.contains("予期しない文字"), "{message} に種別が無い");
    }

    // ── fixture ────────────────────────────────────────────────────────────

    #[test]
    fn reads_a_graphql_response_fixture() {
        let value = parse_json(GRAPHQL_RESPONSE).expect("fixture は読める");
        let repository = value
            .get("data")
            .and_then(|found| found.get("r0"))
            .expect("r0 がある");
        assert_eq!(
            repository.get("nameWithOwner").and_then(JsonValue::as_str),
            Some("example-org/alpha")
        );
        assert_eq!(
            repository
                .get("defaultBranchRef")
                .and_then(|found| found.get("name"))
                .and_then(JsonValue::as_str),
            Some("main")
        );
        assert_eq!(
            repository
                .get("defaultBranchRef")
                .and_then(|found| found.get("target"))
                .and_then(|found| found.get("statusCheckRollup"))
                .and_then(|found| found.get("state"))
                .and_then(JsonValue::as_str),
            Some("SUCCESS")
        );
    }

    /// エスケープを含む PR タイトルと、`false` / 数がそのまま読める。
    #[test]
    fn reads_the_pull_requests_of_the_fixture() {
        let value = parse_json(GRAPHQL_RESPONSE).expect("fixture は読める");
        let pull_request = value
            .get("data")
            .and_then(|found| found.get("r0"))
            .and_then(|found| found.get("pullRequests"))
            .and_then(|found| found.get("nodes"))
            .and_then(JsonValue::as_array)
            .and_then(|nodes| nodes.first())
            .expect("PR が 1 件ある");
        assert_eq!(
            pull_request.get("title").and_then(JsonValue::as_str),
            Some("Add login \"flow\"")
        );
        assert_eq!(
            pull_request
                .get("number")
                .and_then(JsonValue::as_number)
                .and_then(JsonNumber::as_u64),
            Some(12_u64)
        );
        assert_eq!(
            pull_request.get("isDraft").and_then(JsonValue::as_bool),
            Some(false)
        );
    }

    /// 🔑 取れなかったリポは `null` で返り、`errors` に理由が入る。
    /// **終了コードで捨てると、この応答の成功分まで消える**（ADR 0003 決定 5）。
    #[test]
    fn reads_the_null_repository_and_the_errors_of_the_fixture() {
        let value = parse_json(GRAPHQL_RESPONSE).expect("fixture は読める");
        let repositories = value.get("data").expect("data がある");
        assert!(repositories.get("r1").is_some_and(JsonValue::is_null));
        assert!(
            repositories
                .get("r2")
                .and_then(|found| found.get("defaultBranchRef"))
                .is_some_and(JsonValue::is_null)
        );
        let failure = value
            .get("errors")
            .and_then(JsonValue::as_array)
            .and_then(|items| items.first())
            .expect("errors が 1 件ある");
        assert_eq!(
            failure.get("type").and_then(JsonValue::as_str),
            Some("NOT_FOUND")
        );
        assert_eq!(
            failure
                .get("path")
                .and_then(JsonValue::as_array)
                .and_then(|items| items.first())
                .and_then(JsonValue::as_str),
            Some("r1")
        );
    }

    /// 応答の順（`r0` `r1` `r2`）が保たれる。並べ替えると投げた順との対応が消える。
    #[test]
    fn keeps_the_order_of_the_response() {
        let value = parse_json(GRAPHQL_RESPONSE).expect("fixture は読める");
        let members = value
            .get("data")
            .and_then(JsonValue::as_object)
            .expect("data はオブジェクトである");
        let names: alloc::vec::Vec<&str> = members.iter().map(|member| member.0.as_str()).collect();
        assert_eq!(names, alloc::vec!["r0", "r1", "r2"]);
    }
}
