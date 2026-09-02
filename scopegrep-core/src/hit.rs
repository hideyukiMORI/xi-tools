//! 検索が見つけた1件。

use alloc::string::String;

use crate::column::Column;
use crate::line_number::LineNumber;
use crate::scope_path::ScopePath;

/// 検索が見つけた1件。**行番号だけでなく所属を持つ**ことが `grep` との違いである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    path: ScopePath,
    line: LineNumber,
    column: Column,
    value: String,
}

impl Hit {
    /// 1件を作る。
    pub(crate) fn new(path: ScopePath, line: LineNumber, column: Column, value: String) -> Self {
        Self {
            path,
            line,
            column,
            value,
        }
    }

    /// 値が属する場所。
    #[must_use]
    pub fn path(&self) -> &ScopePath {
        &self.path
    }

    /// 値のある行（1 始まり）。
    #[must_use]
    pub fn line(&self) -> LineNumber {
        self.line
    }

    /// 一致が始まる桁（1 始まり・文字数）。
    #[must_use]
    pub fn column(&self) -> Column {
        self.column
    }

    /// 一致した行のスカラーテキスト（原文のまま）。
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}
