//! 1行ぶんのスカラー値。`Document` の内部表現。

use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::column::Column;
use crate::hit::Hit;
use crate::line_number::LineNumber;
use crate::query::Query;
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

    /// 条件に当たるなら1件を返す。**1行につき最大1件**（`grep` と同じ行単位）。
    ///
    /// 所属の絞り込み（`--scope`）は照合より先に見る。**所属が外なら、そもそも探さない**。
    pub(crate) fn find(&self, query: &Query) -> Option<Hit> {
        if !query.covers(&self.path) {
            return None;
        }
        let column = self
            .column
            .locate(&self.text, query.needle(), query.case())?;
        Some(Hit::in_value(
            self.path.clone(),
            self.line,
            column,
            self.text.clone(),
        ))
    }
}
