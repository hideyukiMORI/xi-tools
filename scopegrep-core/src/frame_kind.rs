//! 入れ子の1段が、マッピングかシーケンスか。

use alloc::string::String;

/// 入れ子の1段の種類と、その段で今読んでいる項目。
///
/// 🔑 bool の組み合わせで表さない（RS-002）。「マッピングなのに索引を持つ」ような
/// 状態を**書けなくする**ために enum にしている。
#[derive(Debug, Clone)]
pub(crate) enum FrameKind {
    /// ブロックマッピング。今のキー（まだ何も読んでいなければ `None`）。
    Mapping {
        /// 直近に読んだキー。
        key: Option<String>,
    },
    /// ブロックシーケンス。今の索引（まだ要素を読んでいなければ `None`）。
    Sequence {
        /// 直近に読んだ要素の索引（0 始まり）。
        index: Option<usize>,
    },
}
