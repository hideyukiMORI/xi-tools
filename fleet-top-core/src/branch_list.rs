//! 取得できた枝の一覧と、それが全部かどうか。

use alloc::vec::Vec;

use crate::freshness::Freshness;
use crate::remote_branch::RemoteBranch;
use crate::stale_count::StaleCount;

/// GraphQL で取得した枝の一覧。
///
/// 🔴 **「取れた枝」と「全部取れたか」を一緒に持つ。** 別々に持ち回ると、
/// 100 本で切られたことを忘れて数えた本数を答える経路ができる。
/// [`BranchList::stale`] が `Truncated` を返せるのは、この 2 つが同じ型に居るからである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BranchList {
    branches: Vec<RemoteBranch>,
    truncated: bool,
}

impl BranchList {
    /// 取れた枝と、切り詰められたかどうかから作る。
    pub(crate) fn new(branches: Vec<RemoteBranch>, truncated: bool) -> Self {
        Self {
            branches,
            truncated,
        }
    }

    /// 既定枝を除いた枝のうち、古いものの数。
    ///
    /// 枝が切り詰められていれば [`StaleCount::Truncated`]。
    /// `default_branch` が `None`（既定枝が無いリポジトリ）なら全ての枝を数える。
    pub(crate) fn stale(&self, default_branch: Option<&str>, freshness: &Freshness) -> StaleCount {
        if self.truncated {
            return StaleCount::Truncated;
        }
        let count = self
            .branches
            .iter()
            .filter(|branch| Some(branch.name()) != default_branch)
            .filter(|branch| freshness.is_stale(branch.last_commit()))
            .count();
        StaleCount::Known(u32::try_from(count).unwrap_or(u32::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::BranchList;
    use crate::day::Day;
    use crate::freshness::Freshness;
    use crate::remote_branch::RemoteBranch;
    use crate::stale_count::StaleCount;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    fn day(text: &str) -> Day {
        Day::parse_iso8601(text).expect("読めるはずである")
    }

    fn branch(name: &str, committed: &str) -> RemoteBranch {
        RemoteBranch::new(String::from(name), day(committed))
    }

    fn freshness(stale_days: u32) -> Freshness {
        Freshness::new(day("2026-09-02"), stale_days)
    }

    /// 既定枝（`main`）は古くても数えない。
    fn sample() -> Vec<RemoteBranch> {
        vec![
            branch("main", "2026-01-01"),
            branch("feat/login", "2026-08-02"),
            branch("feat/logout", "2026-08-03"),
            branch("fresh", "2026-09-02"),
        ]
    }

    #[test]
    fn counts_only_branches_older_than_the_limit() {
        let list = BranchList::new(sample(), false);
        assert_eq!(
            list.stale(Some("main"), &freshness(30_u32)),
            StaleCount::Known(1_u32)
        );
    }

    /// 既定枝が無いリポジトリでは全ての枝が対象になる。
    #[test]
    fn without_a_default_branch_every_branch_counts() {
        let list = BranchList::new(sample(), false);
        assert_eq!(
            list.stale(None, &freshness(30_u32)),
            StaleCount::Known(2_u32)
        );
    }

    /// 切り詰められていたら数を答えない。
    #[test]
    fn a_truncated_list_cannot_be_counted() {
        let list = BranchList::new(sample(), true);
        assert_eq!(
            list.stale(Some("main"), &freshness(30_u32)),
            StaleCount::Truncated
        );
    }

    #[test]
    fn an_empty_list_counts_zero() {
        let list = BranchList::new(Vec::new(), false);
        assert_eq!(
            list.stale(None, &freshness(30_u32)),
            StaleCount::Known(0_u32)
        );
    }

    /// 未来の日付は数えない。
    #[test]
    fn a_future_commit_is_not_stale() {
        let list = BranchList::new(vec![branch("ahead", "2026-09-03")], false);
        assert_eq!(
            list.stale(None, &freshness(0_u32)),
            StaleCount::Known(0_u32)
        );
    }
}
