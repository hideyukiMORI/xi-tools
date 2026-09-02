//! 引数の読み方が分からなかったこと。

use core::fmt;

/// 引数の読み方が分からなかったこと。
///
/// 🔑 理由の内訳を持たない。使い方を出すのが唯一の対応であり、
/// **枝を増やしても出力は1つ**だからである（RS-004: `Option` と同じく、
/// 型が表す意味は一つに固定する）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UsageError;

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("usage: scopegrep [--json] <needle> <path>...")
    }
}
