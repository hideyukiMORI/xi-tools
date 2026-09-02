//! 次の行を待っている状態。

use crate::pending_block::PendingBlock;
use crate::pending_flow::PendingFlow;

/// 「次の行も同じ値の続きである」と分かっている状態。
///
/// 🔑 ブロックスカラーと複数行フローを**1つの場所**で持つ（RS-002）。
/// 別々の `Option` にすると「両方が同時に開いている」という起こりえない状態を
/// 書けてしまう。どちらか一方しか開かないことを型で示す。
#[derive(Debug, Clone)]
pub(crate) enum Continuation {
    /// ブロックスカラー（`|` / `>`）の内容を読んでいる。
    Block(PendingBlock),
    /// フロー記法（`[` / `{`）が閉じるのを待っている。
    Flow(PendingFlow),
}
