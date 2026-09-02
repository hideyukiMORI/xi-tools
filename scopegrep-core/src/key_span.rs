//! 読み取ったキーと、その後ろの `:` の位置。

use alloc::string::String;

/// マッピングのキーと、そのキーを終わらせる `:` のバイト位置。
///
/// 🔑 「キー」と「位置」を裸のタプルで持ち回らない（RS-006）。
#[derive(Debug, Clone)]
pub(crate) struct KeySpan {
    text: String,
    colon: usize,
}

impl KeySpan {
    /// キーと `:` の位置から作る。
    pub(crate) fn new(text: String, colon: usize) -> Self {
        Self { text, colon }
    }

    /// キー。クォートは外すが、中身のエスケープは解かない。
    pub(crate) fn into_text(self) -> String {
        self.text
    }

    /// キーを終わらせる `:` のバイト位置。
    pub(crate) fn colon(&self) -> usize {
        self.colon
    }
}
