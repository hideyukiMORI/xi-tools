//! 1行ぶんのスカラー値。`Document` の内部表現。

use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::column::Column;
use crate::hit::Hit;
use crate::line_number::LineNumber;
use crate::scope_path::ScopePath;

/// 1行ぶんのスカラー値と、その所属。
///
/// 🔑 木ではなく**平坦な表**で持つ。検索は「どの値がどこに属するか」しか要らず、
/// 表なら出現順がそのまま出力順になる（RS-016）。
#[derive(Debug, Clone)]
pub(crate) struct ScalarLine {
    path: ScopePath,
    line: LineNumber,
    column: Column,
    text: String,
}

impl ScalarLine {
    /// 1行を作る。`column` は値の先頭の桁。
    pub(crate) fn new(path: ScopePath, line: LineNumber, column: Column, text: String) -> Self {
        Self {
            path,
            line,
            column,
            text,
        }
    }

    /// ラベル表を当てはめた行を返す。
    pub(crate) fn with_labels(self, labels: &BTreeMap<String, String>) -> Self {
        Self {
            path: self.path.with_labels(labels),
            line: self.line,
            column: self.column,
            text: self.text,
        }
    }

    /// `needle` を含むなら1件を返す。**1行につき最大1件**（`grep` と同じ行単位）。
    pub(crate) fn find(&self, needle: &str) -> Option<Hit> {
        let index = self.text.find(needle)?;
        let chars_before = self.text.get(..index).unwrap_or("").chars().count();
        Some(Hit::new(
            self.path.clone(),
            self.line,
            self.column.shift(chars_before),
            self.text.clone(),
        ))
    }
}
