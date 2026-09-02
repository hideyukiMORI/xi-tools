//! 読んでいる途中の、複数行にまたがるフロー記法。

use crate::flow_scan::FlowScan;
use crate::line_number::LineNumber;
use crate::scope_path::ScopePath;

/// 括弧が閉じるのを待っているフロー記法。
///
/// 🔑 **開いた行**を覚える。閉じないまま終わったときに「何行目が閉じていないか」を
/// 言えなければ、エラーは「このファイルは読めない」以上のことを言えない。
#[derive(Debug, Clone)]
pub(crate) struct PendingFlow {
    path: ScopePath,
    parent_indent: usize,
    line: LineNumber,
    scan: FlowScan,
}

impl PendingFlow {
    /// 所属パス・親の桁・開いた行・走査状態から作る。
    pub(crate) fn new(
        path: ScopePath,
        parent_indent: usize,
        line: LineNumber,
        scan: FlowScan,
    ) -> Self {
        Self {
            path,
            parent_indent,
            line,
            scan,
        }
    }

    /// 内容の所属パス。**続きの各行がこのパスを持つ**（ブロックスカラーと同じ）。
    pub(crate) fn path(&self) -> &ScopePath {
        &self.path
    }

    /// このフローを導入したキー（または `-`）の桁。続きはこれより深くなければならない。
    pub(crate) fn parent_indent(&self) -> usize {
        self.parent_indent
    }

    /// 括弧を開いた行。閉じなかったときに報告する行番号である。
    pub(crate) fn line(&self) -> LineNumber {
        self.line
    }

    /// 続きの行を読み進める。閉じたら、その行の中の**閉じ括弧の次のバイト位置**。
    pub(crate) fn advance(&mut self, text: &str) -> Option<usize> {
        self.scan.advance(text)
    }
}
