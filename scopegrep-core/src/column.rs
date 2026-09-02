//! 1 始まりの桁（文字数）。

/// 1 始まりの桁。
///
/// **単位はバイトではなく文字**である（`grep -b` ではなくエディタの桁に合わせる）。
/// フィールドは非公開で、生成経路は [`Column::new`] だけである（RS-001）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Column(u32);

impl Column {
    /// 桁を作る。0 桁目は存在しないので拒否する。
    #[must_use]
    pub fn new(value: u32) -> Option<Self> {
        (value > 0_u32).then_some(Self(value))
    }

    /// 生の値。
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }

    /// 先行する文字数から桁を作る。`chars_before` が 0 なら 1 桁目。
    pub(crate) fn after(chars_before: usize) -> Self {
        Self(u32::try_from(chars_before.saturating_add(1_usize)).unwrap_or(u32::MAX))
    }

    /// この桁から `chars` 文字ぶん右へ動かす。
    pub(crate) fn shift(self, chars: usize) -> Self {
        Self(
            self.0
                .saturating_add(u32::try_from(chars).unwrap_or(u32::MAX)),
        )
    }
}

impl core::fmt::Display for Column {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Column;

    #[test]
    fn zero_is_not_a_column() {
        assert!(Column::new(0_u32).is_none());
    }

    #[test]
    fn after_counts_from_one() {
        assert_eq!(Column::after(0_usize).get(), 1_u32);
        assert_eq!(Column::after(12_usize).get(), 13_u32);
    }

    #[test]
    fn shift_moves_right() {
        assert_eq!(Column::after(0_usize).shift(4_usize).get(), 5_u32);
    }
}
