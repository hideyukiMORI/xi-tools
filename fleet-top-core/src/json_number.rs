//! JSON の数。字句を原文のまま持つ。

use alloc::string::String;

/// JSON の数。**字句を原文のまま保つ**。
///
/// 🔴 `f64` に落とさない。GitHub の応答には件数・PR 番号・行番号が整数で入り、
/// `f64` を経由すると 2^53 を超えた時点で**黙って**丸まる。JSON の文法は整数と
/// 小数を区別しないので、区別を捨てるのは読み手の仕事ではない。
///
/// フィールドは非公開で、生成経路はこのクレートの中だけである（RS-001 / RS-003）。
/// 字句は [`crate::json_parser::parse_json`] が文法を検証した後の形しか入らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonNumber(String);

impl JsonNumber {
    /// 文法を検証済みの字句から作る。
    pub(crate) fn new(lexeme: String) -> Self {
        Self(lexeme)
    }

    /// 原文の字句。`-0` は `-0`、`1.50` は `1.50` のまま返る。
    #[must_use]
    pub fn lexeme(&self) -> &str {
        &self.0
    }

    /// 符号なし整数として読む。
    ///
    /// 小数点・指数・負号があるとき、`u64` に収まらないときは `None`。
    /// **`1.0` を 1 として読まない**（丸めが起きた事実を呼び手から隠さないため）。
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        if self.0.contains(['.', 'e', 'E', '-']) {
            return None;
        }
        self.0.parse::<u64>().ok()
    }

    /// 符号付き整数として読む。小数点・指数があるとき、`i64` に収まらないときは `None`。
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        if self.0.contains(['.', 'e', 'E']) {
            return None;
        }
        self.0.parse::<i64>().ok()
    }

    /// 浮動小数点数として読む。
    ///
    /// 字句は文法で検証済みなので、実際にはここが `None` を返す入力は無い。
    /// それでも `f64` を直接返さないのは、**失敗経路を偽の値で埋めないため**である
    /// （`unwrap` は書けないので、代わりに 0 や NaN を返すことになる。それは嘘である）。
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        self.0.parse::<f64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::JsonNumber;
    use alloc::string::String;

    fn number(lexeme: &str) -> JsonNumber {
        JsonNumber::new(String::from(lexeme))
    }

    #[test]
    fn keeps_the_lexeme_verbatim() {
        assert_eq!(number("-0").lexeme(), "-0");
        assert_eq!(number("1.50").lexeme(), "1.50");
    }

    #[test]
    fn reads_unsigned_integers() {
        assert_eq!(number("0").as_u64(), Some(0_u64));
        assert_eq!(number("12").as_u64(), Some(12_u64));
    }

    /// `u64` の上限ちょうどは読め、1 つ超えたら読めない。
    #[test]
    fn stops_at_the_unsigned_boundary() {
        assert_eq!(number("18446744073709551615").as_u64(), Some(u64::MAX));
        assert_eq!(number("18446744073709551616").as_u64(), None);
    }

    #[test]
    fn refuses_non_integers_as_unsigned() {
        assert_eq!(number("1.0").as_u64(), None);
        assert_eq!(number("1e5").as_u64(), None);
        assert_eq!(number("1E5").as_u64(), None);
        assert_eq!(number("-1").as_u64(), None);
    }

    #[test]
    fn reads_signed_integers() {
        assert_eq!(number("-1").as_i64(), Some(-1_i64));
        assert_eq!(number("9223372036854775807").as_i64(), Some(i64::MAX));
        assert_eq!(number("9223372036854775808").as_i64(), None);
        assert_eq!(number("1.0").as_i64(), None);
        assert_eq!(number("1e5").as_i64(), None);
    }

    #[test]
    fn reads_floating_point() {
        assert_eq!(number("1.5").as_f64(), Some(1.5_f64));
        assert_eq!(number("1.5E-3").as_f64(), Some(0.0015_f64));
        assert_eq!(number("-0").as_f64(), Some(-0.0_f64));
    }
}
