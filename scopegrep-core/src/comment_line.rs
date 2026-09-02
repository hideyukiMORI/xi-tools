//! 1行ぶんのコメント。`Document` の内部表現。

use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::column::Column;
use crate::hit::Hit;
use crate::line_number::LineNumber;
use crate::query::Query;
use crate::scope_path::ScopePath;

/// コメント1件と、その所属。
///
/// 🔑 ここでいう所属は「**そのコメントがどの入れ子の中に書かれたか**」であって、
/// 「誰の説明か」ではない。後者は推測になる。設計メモ「D-2 実測」に記録したとおり、
/// `tree-sitter-yaml` の構文木では**ステップ直前のコメントが前のステップの子になる**。
/// 桁だけを見る機械的な規則なら、そのような取り違えが起きない。
///
/// スカラー値と同じく**平坦な表**で持つ。行番号順がそのまま出力順になる（RS-016）。
#[derive(Debug, Clone)]
pub(crate) struct CommentLine {
    path: ScopePath,
    line: LineNumber,
    column: Column,
    text: String,
}

impl CommentLine {
    /// コメント1件を作る。`column` は `#` の桁、`text` は `#` から行末までの原文。
    pub(crate) fn new(path: ScopePath, line: LineNumber, column: Column, text: String) -> Self {
        Self {
            path,
            line,
            column,
            text,
        }
    }

    /// ラベル表を当てはめた行を返す。
    ///
    /// 🔑 値と同じ表を当てる。同じ `steps[3]` が、値のヒットではラベル付きで、
    /// コメントのヒットでは索引だけ、という食い違いを作らないため。
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
    /// 🔑 所属の絞り込みは値と**同じ規則**でコメントにも当てる。
    /// 「値には効くがコメントには効かない旗」を作らない。
    pub(crate) fn find(&self, query: &Query) -> Option<Hit> {
        if !query.covers(&self.path) {
            return None;
        }
        let column = self
            .column
            .locate(&self.text, query.needle(), query.case())?;
        Some(Hit::in_comment(
            self.path.clone(),
            self.line,
            column,
            self.text.clone(),
        ))
    }
}
