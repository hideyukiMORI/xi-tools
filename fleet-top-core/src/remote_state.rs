//! GitHub 側から見たリポジトリの状態。

use alloc::string::String;

use crate::branch_list::BranchList;
use crate::ci_state::CiState;
use crate::freshness::Freshness;
use crate::stale_count::StaleCount;

/// GitHub 側から見た 1 リポジトリの状態。
///
/// フィールドは非公開で、生成経路は [`crate::graphql::parse_response`] だけである
/// （RS-001 / RS-003）。**GraphQL の応答を保持しない**——読んだ結果だけを持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteState {
    default_branch: Option<String>,
    ci: CiState,
    open_pull_requests: u32,
    branches: BranchList,
}

impl RemoteState {
    /// 読み取った各値から作る。
    pub(crate) fn new(
        default_branch: Option<String>,
        ci: CiState,
        open_pull_requests: u32,
        branches: BranchList,
    ) -> Self {
        Self {
            default_branch,
            ci,
            open_pull_requests,
            branches,
        }
    }

    /// 既定枝の名前。**空のリポジトリでは `None`**（`defaultBranchRef` が `null`）。
    #[must_use]
    pub fn default_branch(&self) -> Option<&str> {
        self.default_branch.as_deref()
    }

    /// 既定枝の先頭コミットの CI の状態。既定枝が無ければ [`CiState::Absent`]。
    #[must_use]
    pub fn ci(&self) -> CiState {
        self.ci
    }

    /// open な PR の数。
    #[must_use]
    pub fn open_pull_requests(&self) -> u32 {
        self.open_pull_requests
    }

    /// 既定枝**以外**の枝のうち、`freshness` より古いものの数。
    ///
    /// 枝が 100 本を超えて切り詰められていれば [`StaleCount::Truncated`]。
    #[must_use]
    pub fn stale_branches(&self, freshness: &Freshness) -> StaleCount {
        self.branches
            .stale(self.default_branch.as_deref(), freshness)
    }
}

#[cfg(test)]
mod tests {
    use super::RemoteState;
    use crate::branch_list::BranchList;
    use crate::ci_state::CiState;
    use crate::day::Day;
    use crate::freshness::Freshness;
    use crate::remote_branch::RemoteBranch;
    use crate::stale_count::StaleCount;
    use alloc::string::String;
    use alloc::vec;

    fn day(text: &str) -> Day {
        Day::parse_iso8601(text).expect("読めるはずである")
    }

    #[test]
    fn answers_what_it_was_built_from() {
        let branches = BranchList::new(
            vec![
                RemoteBranch::new(String::from("main"), day("2026-01-01")),
                RemoteBranch::new(String::from("old"), day("2026-01-01")),
            ],
            false,
        );
        let state = RemoteState::new(
            Some(String::from("main")),
            CiState::Failure,
            3_u32,
            branches,
        );
        assert_eq!(state.default_branch(), Some("main"));
        assert_eq!(state.ci(), CiState::Failure);
        assert_eq!(state.open_pull_requests(), 3_u32);
        assert_eq!(
            state.stale_branches(&Freshness::new(day("2026-09-02"), 30_u32)),
            StaleCount::Known(1_u32)
        );
    }

    /// 既定枝が無い（空の）リポジトリでは、全ての枝が古さの対象になる。
    #[test]
    fn without_a_default_branch_every_branch_is_counted() {
        let branches = BranchList::new(
            vec![RemoteBranch::new(String::from("main"), day("2026-01-01"))],
            false,
        );
        let state = RemoteState::new(None, CiState::Absent, 0_u32, branches);
        assert_eq!(state.default_branch(), None);
        assert_eq!(
            state.stale_branches(&Freshness::new(day("2026-09-02"), 30_u32)),
            StaleCount::Known(1_u32)
        );
    }
}
