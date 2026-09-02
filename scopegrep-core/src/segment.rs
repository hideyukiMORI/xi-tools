//! 所属パスの1要素。

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::{String, ToString};

/// 所属パスの1要素。マッピングのキーか、シーケンスの索引のどちらかである。
///
/// 索引はラベル（要素の `name` の値）を持つことがある。ラベルは YAML の規格ではなく
/// GitHub Actions / Ansible / Kubernetes に共通する慣習なので、
/// **JSON Pointer には混ぜない**（設計メモ D-1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Segment {
    /// マッピングのキー。
    Key(String),
    /// シーケンスの索引（0 始まり）と、その要素のラベル。
    Index {
        /// 0 始まりの索引。
        index: usize,
        /// 要素の `name` の値。無ければ索引だけを出す（設計メモ D-3）。
        label: Option<String>,
    },
}

impl Segment {
    /// RFC 6901 の参照トークン。`~` と `/` を退避する。
    pub(crate) fn pointer_token(&self) -> String {
        match *self {
            Self::Key(ref key) => key.replace('~', "~0").replace('/', "~1"),
            Self::Index { index, .. } => index.to_string(),
        }
    }

    /// ラベルを付け直した要素を返す。キーはそのまま。
    pub(crate) fn with_label(self, label: Option<String>) -> Self {
        match self {
            Self::Key(key) => Self::Key(key),
            Self::Index { index, .. } => Self::Index { index, label },
        }
    }

    /// 人向けの表示。`first` が真なら先頭要素として区切りを付けない。
    pub(crate) fn render(&self, first: bool, after_label: bool) -> String {
        match *self {
            Self::Key(ref key) => {
                let separator = if after_label {
                    " ."
                } else if first {
                    ""
                } else {
                    "."
                };
                format!("{separator}{}", render_key(key))
            }
            Self::Index {
                index,
                label: Some(ref text),
            } => format!("[{index}] \"{}\"", escape(text)),
            Self::Index { index, label: None } => format!("[{index}]"),
        }
    }

    /// この要素の直後にラベルが出たか（次のキーの区切りが ` .` になる）。
    pub(crate) fn ends_with_label(&self) -> bool {
        match *self {
            Self::Key(_) => false,
            Self::Index { ref label, .. } => label.is_some(),
        }
    }
}

/// キーを人向けに描く。`[A-Za-z0-9_-]` だけなら裸、そうでなければ `"…"` で囲む。
fn render_key(key: &str) -> String {
    let plain = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if plain {
        key.to_owned()
    } else {
        format!("\"{}\"", escape(key))
    }
}

/// `"` と `\` を `\` で退避する。**`\` を先に置き換える**（後だと二重に効く）。
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::{Segment, escape, render_key};
    use alloc::borrow::ToOwned;

    #[test]
    fn pointer_token_escapes_rfc6901() {
        let segment = Segment::Key("a~b/c".to_owned());
        assert_eq!(segment.pointer_token(), "a~0b~1c");
    }

    #[test]
    fn plain_key_is_bare() {
        assert_eq!(render_key("frontend-check"), "frontend-check");
    }

    #[test]
    fn odd_key_is_quoted() {
        assert_eq!(render_key("weird key"), "\"weird key\"");
    }

    #[test]
    fn escape_handles_backslash_first() {
        assert_eq!(escape("a\\\"b"), "a\\\\\\\"b");
    }
}
