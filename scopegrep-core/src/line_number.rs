//! 1 始まりの行番号。

/// 1 始まりの行番号。
///
/// フィールドは非公開で、生成経路は [`LineNumber::new`] だけである（RS-001）。
/// **0 行目は存在しない**ので、0 からは作れない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineNumber(u32);

impl LineNumber {
    /// 行番号を作る。0 は行番号ではないので拒否する。
    #[must_use]
    pub fn new(value: u32) -> Option<Self> {
        (value > 0_u32).then_some(Self(value))
    }

    /// 生の値。
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }

    /// 最初の行。走査の開始点として使う。
    pub(crate) fn first() -> Self {
        Self(1_u32)
    }

    /// 次の行。`u32` の上限で頭打ちにする（巻き戻さない・RS-017）。
    pub(crate) fn advance(self) -> Self {
        Self(self.0.saturating_add(1_u32))
    }
}

impl core::fmt::Display for LineNumber {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::LineNumber;

    #[test]
    fn zero_is_not_a_line_number() {
        assert!(LineNumber::new(0_u32).is_none());
    }

    #[test]
    fn keeps_the_value() {
        assert_eq!(LineNumber::new(33_u32).map(LineNumber::get), Some(33_u32));
    }

    #[test]
    fn advance_moves_one_line() {
        assert_eq!(LineNumber::first().advance().get(), 2_u32);
    }
}
