//! JSON の値。

use alloc::string::String;
use alloc::vec::Vec;

use crate::json_number::JsonNumber;

/// JSON の値（RFC 8259 の 6 種）。
///
/// 🔴 オブジェクトは `BTreeMap` ではなく**挿入順の `Vec`** である（RS-016）。
/// 反復順を決定的にするだけなら `BTreeMap` でよいが、それでは**応答が返してきた順**が
/// 消える。GraphQL のエイリアス（`r0` `r1` …）は投げた順に返るので、その順に意味がある。
/// 重複キーも両方残す（[`JsonValue::get`] は最後のものを返す）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonValue {
    /// `null`。
    Null,
    /// `true` / `false`。
    Bool(bool),
    /// 数。字句のまま持つ（[`JsonNumber`]）。
    Number(JsonNumber),
    /// 文字列。エスケープは解いた後の中身。
    String(String),
    /// 配列。
    Array(Vec<JsonValue>),
    /// オブジェクト。**挿入順**の `(キー, 値)` の並び。
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// オブジェクトのメンバーを引く。オブジェクトでなければ `None`。
    ///
    /// 重複キーがあるときは**最後のもの**を返す（後から書かれた値が勝つ）。
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        self.as_object()?
            .iter()
            .rev()
            .find(|member| member.0.as_str() == key)
            .map(|member| &member.1)
    }

    /// 文字列としての中身。文字列でなければ `None`。
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Self::String(ref text) => Some(text),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::Array(_) | Self::Object(_) => None,
        }
    }

    /// 真偽値としての中身。真偽値でなければ `None`。
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Bool(flag) => Some(flag),
            Self::Null | Self::Number(_) | Self::String(_) | Self::Array(_) | Self::Object(_) => {
                None
            }
        }
    }

    /// 配列としての中身。配列でなければ `None`。
    #[must_use]
    pub fn as_array(&self) -> Option<&[Self]> {
        match *self {
            Self::Array(ref items) => Some(items),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::String(_) | Self::Object(_) => {
                None
            }
        }
    }

    /// オブジェクトとしての中身。オブジェクトでなければ `None`。
    #[must_use]
    pub fn as_object(&self) -> Option<&[(String, Self)]> {
        match *self {
            Self::Object(ref members) => Some(members),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::String(_) | Self::Array(_) => None,
        }
    }

    /// 数としての中身。数でなければ `None`。
    #[must_use]
    pub fn as_number(&self) -> Option<&JsonNumber> {
        match *self {
            Self::Number(ref number) => Some(number),
            Self::Null | Self::Bool(_) | Self::String(_) | Self::Array(_) | Self::Object(_) => None,
        }
    }

    /// `null` か。
    ///
    /// 🔑 GraphQL は「取れなかったリポ」を `null` で返す。`Option` に潰さずに
    /// `null` を値として持つのは、**在るのに `null`** と**キーが無い**を区別するためである。
    #[must_use]
    pub fn is_null(&self) -> bool {
        match *self {
            Self::Null => true,
            Self::Bool(_)
            | Self::Number(_)
            | Self::String(_)
            | Self::Array(_)
            | Self::Object(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JsonValue;
    use crate::json_number::JsonNumber;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    fn object(members: Vec<(&str, JsonValue)>) -> JsonValue {
        JsonValue::Object(
            members
                .into_iter()
                .map(|(key, value)| (String::from(key), value))
                .collect(),
        )
    }

    #[test]
    fn accessors_answer_only_for_their_own_shape() {
        let text = JsonValue::String(String::from("main"));
        assert_eq!(text.as_str(), Some("main"));
        assert_eq!(text.as_bool(), None);
        assert_eq!(text.as_array(), None);
        assert_eq!(text.as_object(), None);
        assert_eq!(text.as_number(), None);
        assert!(!text.is_null());
    }

    #[test]
    fn null_is_null_and_nothing_else() {
        let value = JsonValue::Null;
        assert!(value.is_null());
        assert_eq!(value.as_str(), None);
        assert_eq!(value.as_bool(), None);
        assert_eq!(value.as_array(), None);
        assert_eq!(value.as_object(), None);
        assert_eq!(value.as_number(), None);
        assert_eq!(value.get("anything"), None);
    }

    #[test]
    fn reads_bool_number_and_array() {
        assert_eq!(JsonValue::Bool(true).as_bool(), Some(true));
        let number = JsonValue::Number(JsonNumber::new(String::from("7")));
        assert_eq!(number.as_number().and_then(JsonNumber::as_u64), Some(7_u64));
        let array = JsonValue::Array(vec![JsonValue::Null]);
        assert_eq!(array.as_array().map(<[JsonValue]>::len), Some(1_usize));
        assert!(array.as_object().is_none());
    }

    #[test]
    fn get_reads_a_member() {
        let value = object(vec![("name", JsonValue::String(String::from("main")))]);
        assert_eq!(value.get("name").and_then(JsonValue::as_str), Some("main"));
        assert_eq!(value.get("missing"), None);
    }

    /// 重複キーは両方残り、`get` は**最後の**ものを返す。
    #[test]
    fn duplicate_keys_resolve_to_the_last_one() {
        let value = object(vec![
            ("state", JsonValue::String(String::from("PENDING"))),
            ("state", JsonValue::String(String::from("SUCCESS"))),
        ]);
        assert_eq!(
            value.as_object().map(<[(String, JsonValue)]>::len),
            Some(2_usize)
        );
        assert_eq!(
            value.get("state").and_then(JsonValue::as_str),
            Some("SUCCESS")
        );
    }
}
