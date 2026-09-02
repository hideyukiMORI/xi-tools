//! 検索が見つけた1件。

use alloc::string::String;

use crate::column::Column;
use crate::hit_kind::HitKind;
use crate::line_number::LineNumber;
use crate::scope_path::ScopePath;

/// 検索が見つけた1件。**行番号だけでなく所属と種別を持つ**ことが `grep` との違いである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    path: ScopePath,
    line: LineNumber,
    column: Column,
    value: String,
    kind: HitKind,
}

impl Hit {
    /// 設定値の中の一致を1件作る。
    ///
    /// 🔑 種別ごとに入口を分けてある。`kind` を引数で受け取る形にすると、
    /// **呼ぶ側が種別を取り違えても型が助けてくれない**。
    pub(crate) fn in_value(
        path: ScopePath,
        line: LineNumber,
        column: Column,
        value: String,
    ) -> Self {
        Self {
            path,
            line,
            column,
            value,
            kind: HitKind::Value,
        }
    }

    /// コメントの中の一致を1件作る。
    pub(crate) fn in_comment(
        path: ScopePath,
        line: LineNumber,
        column: Column,
        value: String,
    ) -> Self {
        Self {
            path,
            line,
            column,
            value,
            kind: HitKind::Comment,
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

    /// 一致した行のテキスト（原文のまま）。
    ///
    /// 値のヒットならスカラーテキスト、コメントのヒットなら `#` から行末までである。
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// 設定値の中の一致か、コメントの中の一致か。
    ///
    /// 🔴 **この区別を落として表示しないこと。** 落とした瞬間、出力は
    /// 行ベースの検索と同じ「同じ重みで並んだ5行」に戻る。
    #[must_use]
    pub fn kind(&self) -> HitKind {
        self.kind
    }
}
