//! マッピングの1項目が持つ値。

use crate::block_header::BlockHeader;
use crate::scalar_value::ScalarValue;

/// `key:` の右側にあったもの。
///
/// 🔑 「空」を `Option` で表さない（RS-004）。空の値は**入れ子を受け取れる場所**という
/// 意味を持っており、「値が無い」とは別の事実である。
#[derive(Debug, Clone)]
pub(crate) enum EntryValue {
    /// `key:` で終わっている。null であり、入れ子を受け取れる。
    Empty,
    /// 1行スカラー。
    Scalar(ScalarValue),
    /// ブロックスカラー（`|` / `>`）の始まり。
    Block(BlockHeader),
}
