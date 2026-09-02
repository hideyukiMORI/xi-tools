//! 検索が何を探すか。

/// 検索が何を探すか。**閉じた選択肢なので enum で表す**（RS-002）。
///
/// 🔑 既定は [`SearchScope::Values`] のままにする。コメント内の一致を黙って混ぜると
/// 「行ベースの検索と同じ偽陽性」に戻るので、**呼ぶ側が明示的に選ばない限り返さない**。
/// 選んだ場合も、どちらだったかは [`crate::hit_kind::HitKind`] で必ず区別できる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    /// スカラー値だけを探す。
    Values,
    /// スカラー値とコメントの両方を探す。
    ValuesAndComments,
}
