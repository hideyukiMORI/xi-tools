//! 読み終えた1行が、フロー記法の続きを持ち越すかどうか。
//!
//! 🔑 `Option<FlowScan>` で表さない（RS-004）。「続きが無い」ことは
//! **この行で値が完結した**という事実であって、値の欠落ではない。

use crate::flow_scan::FlowScan;

/// 1行を読み終えた時点のフロー記法の状態。
#[derive(Debug, Clone, Copy)]
pub(crate) enum FlowState {
    /// この行で終わっている（フロー記法でない値も、行内で閉じたフローもここ）。
    Complete,
    /// 括弧が閉じていない。次の行へ持ち越す走査状態を持つ。
    Unclosed(FlowScan),
}
