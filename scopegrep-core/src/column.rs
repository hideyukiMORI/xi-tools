//! 1 始まりの桁（文字数）。

use crate::case_match::CaseMatch;

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

    /// この桁から始まる `text` の中で、`needle` が始まる桁。含まなければ `None`。
    ///
    /// 🔑 バイト位置ではなく**文字数**で数える。ここを間違えると、非 ASCII を含む
    /// 行だけ桁がずれる（設計メモ「D-2 実測」で他のパーサに実際にあった癖である）。
    /// 数え方は `case` によらず同じで、**数えるのは常に原文**である
    /// （[`CaseMatch`] の doc を見よ）。
    pub(crate) fn locate(self, text: &str, needle: &str, case: CaseMatch) -> Option<Self> {
        Some(self.shift(case.locate(text, needle)?))
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
    use crate::case_match::CaseMatch;

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

    #[test]
    fn locate_counts_characters_from_this_column() {
        let column = Column::after(2_usize);
        assert_eq!(
            column
                .locate("あいう x", "x", CaseMatch::Exact)
                .map(Column::get),
            Some(7_u32)
        );
        assert_eq!(column.locate("abc", "z", CaseMatch::Exact), None);
    }

    /// 大文字小文字を無視しても、桁の数え方は変わらない。
    #[test]
    fn locate_keeps_the_same_column_when_folding_case() {
        let column = Column::after(2_usize);
        assert_eq!(
            column
                .locate("あいう X", "x", CaseMatch::Fold)
                .map(Column::get),
            Some(7_u32)
        );
    }
}
