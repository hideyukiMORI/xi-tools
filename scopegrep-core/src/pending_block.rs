//! 読んでいる途中のブロックスカラー。

use crate::scope_path::ScopePath;

/// 読んでいる途中のブロックスカラー（`|` / `>`）。
///
/// 内容の桁は、指示子（`|2`）が無ければ**最初の内容行を見るまで決まらない**。
/// だから `indent` は `Option` である。
#[derive(Debug, Clone)]
pub(crate) struct PendingBlock {
    path: ScopePath,
    parent_indent: usize,
    indent: Option<usize>,
}

impl PendingBlock {
    /// 所属パス・親の桁・（分かっていれば）内容の桁から作る。
    pub(crate) fn new(path: ScopePath, parent_indent: usize, indent: Option<usize>) -> Self {
        Self {
            path,
            parent_indent,
            indent,
        }
    }

    /// 内容の所属パス。内容の各行がこのパスを持つ。
    pub(crate) fn path(&self) -> &ScopePath {
        &self.path
    }

    /// この block を導入したキーの桁。内容はこれより深くなければならない。
    pub(crate) fn parent_indent(&self) -> usize {
        self.parent_indent
    }

    /// 内容の桁。まだ決まっていなければ `None`。
    pub(crate) fn indent(&self) -> Option<usize> {
        self.indent
    }

    /// 最初の内容行を見て、内容の桁を決める。
    pub(crate) fn set_indent(&mut self, indent: usize) {
        self.indent = Some(indent);
    }
}
