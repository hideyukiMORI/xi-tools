//! 追跡枝との差（何コミット進んでいて、何コミット遅れているか）。

/// 上流の追跡枝との差。`# branch.ab +A -B` の A と B。
///
/// 🔑 2 つの数を**組にして持つ**のは、片方だけ更新できる状態を作らないためである
/// （RS-006。`(u32, u32)` のタプルで持ち回ると、どちらが ahead か呼び手が覚える）。
/// `# branch.ab` の行が無いとき（＝上流が無いとき）は 0 と 0 で、
/// 「上流が無い」ことは [`crate::local_state::LocalState::upstream`] が別に持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Divergence {
    ahead: u32,
    behind: u32,
}

impl Divergence {
    /// 進んでいる数と遅れている数から作る。
    pub(crate) fn new(ahead: u32, behind: u32) -> Self {
        Self { ahead, behind }
    }

    /// 上流より進んでいるコミット数。
    pub(crate) fn ahead(self) -> u32 {
        self.ahead
    }

    /// 上流より遅れているコミット数。
    pub(crate) fn behind(self) -> u32 {
        self.behind
    }
}

#[cfg(test)]
mod tests {
    use super::Divergence;

    #[test]
    fn keeps_both_sides_apart() {
        let divergence = Divergence::new(2_u32, 1_u32);
        assert_eq!(divergence.ahead(), 2_u32);
        assert_eq!(divergence.behind(), 1_u32);
    }
}
