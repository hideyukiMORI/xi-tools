//! 古い枝の数（数えられたか、数え切れなかったか）。

/// 古い枝の数。
///
/// 🔴 **`Truncated` を 0 や「不明な数」で表さない。** GraphQL には枝を 100 本しか
/// 頼んでいないので、それを超えたリポジトリでは**数えられない**。数えられなかったことを
/// `Known(100)` のような数に混ぜると、表の `?` が消えて「数えた」ように見える
/// （設計メモ F-5「黙って空にしない」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleCount {
    /// 数えられた。
    Known(u32),
    /// 枝が多すぎて数えられなかった（`refs.totalCount` が取得した本数を超えている）。
    Truncated,
}

#[cfg(test)]
mod tests {
    use super::StaleCount;

    #[test]
    fn a_known_count_is_not_a_truncated_one() {
        assert_ne!(StaleCount::Known(0_u32), StaleCount::Truncated);
        assert_eq!(StaleCount::Known(2_u32), StaleCount::Known(2_u32));
    }
}
