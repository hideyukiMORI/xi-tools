//! GitHub 側について言えること。

use crate::remote_state::RemoteState;

/// 1 リポジトリの GitHub 側について、表に出せること。
///
/// 🔴 **`NotOnGithub` と `Unavailable` を分ける。** origin が GitHub でないリポジトリは
/// **聞いていない**（表では `n/a`）。聞いたのに答えが得られなかったのは `?` である。
/// 一緒にすると、`gh` が落ちている日と、GitHub に置いていないリポジトリが同じ見た目になる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteReport {
    /// GitHub の応答を読めた。
    State(RemoteState),
    /// origin が GitHub でない（または origin が無い）。表では `n/a`。
    NotOnGithub,
    /// 取れなかった（`gh` が無い・失敗した・応答を読めなかった）。表では `?`。
    Unavailable,
}

#[cfg(test)]
mod tests {
    use super::RemoteReport;
    use crate::branch_list::BranchList;
    use crate::ci_state::CiState;
    use crate::remote_state::RemoteState;
    use alloc::vec::Vec;

    #[test]
    fn the_three_answers_are_distinct() {
        let state = RemoteState::new(
            None,
            CiState::Absent,
            0_u32,
            BranchList::new(Vec::new(), false),
        );
        assert_ne!(RemoteReport::State(state), RemoteReport::NotOnGithub);
        assert_ne!(RemoteReport::NotOnGithub, RemoteReport::Unavailable);
    }
}
