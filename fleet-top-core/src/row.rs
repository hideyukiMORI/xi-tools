//! 表の 1 行（1 リポジトリぶん）。

use alloc::format;
use alloc::string::String;

use crate::ci_state::CiState;
use crate::freshness::Freshness;
use crate::head::Head;
use crate::local_report::LocalReport;
use crate::remote_report::RemoteReport;
use crate::stale_count::StaleCount;
use crate::table::COLUMN_COUNT;

/// 表の 1 行。ディレクトリ名と、手元・GitHub の 2 つの報告を持つ。
///
/// 🔑 **行は「取れたもの」ではなく「聞いたもの」を 1 行ずつ持つ。** 失敗したリポジトリの
/// 行を落とさないので、リポジトリの数と行の数が必ず一致する（設計メモ F-5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    name: String,
    local: LocalReport,
    remote: RemoteReport,
}

/// 取れなかったことの印。
const UNKNOWN: &str = "?";
/// 聞いていないことの印（GitHub に無いリポジトリ）。
const NOT_APPLICABLE: &str = "n/a";
/// ゼロ・該当なしの印。
const NOTHING: &str = "-";
/// 枝の上に居ないこと。
const DETACHED: &str = "(detached)";
/// 上流の追跡枝が無いこと。
const NO_UPSTREAM: &str = "(none)";

impl Row {
    /// ディレクトリ名と 2 つの報告から作る。
    #[must_use]
    pub fn new(name: String, local: LocalReport, remote: RemoteReport) -> Self {
        Self {
            name,
            local,
            remote,
        }
    }

    /// ディレクトリ名。表の並び順もこれで決まる（バイト順）。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// この行が確定しているか（`?` を 1 つも出さないか）。
    ///
    /// 🔑 **表に出る文字で判定する。** 「どの場合に `?` が出るか」を条件式で
    /// 書き直すと、表の側を直したときに片方だけ動く。終了コードは
    /// [`Row::is_complete`] が決めるので、ここがずれると嘘の終了コードになる。
    #[must_use]
    pub fn is_complete(&self, freshness: &Freshness) -> bool {
        self.cells(freshness)
            .iter()
            .all(|cell| cell.as_str() != UNKNOWN)
    }

    /// 各列に出す文字列。並びは `REPO BRANCH DIRTY AHEAD/BEHIND PR CI STALE`。
    pub(crate) fn cells(&self, freshness: &Freshness) -> [String; COLUMN_COUNT] {
        [
            self.name.clone(),
            branch_cell(&self.local),
            dirty_cell(&self.local),
            divergence_cell(&self.local),
            pull_request_cell(&self.remote),
            ci_cell(&self.remote),
            stale_cell(&self.remote, freshness),
        ]
    }
}

/// 0 は `-`、それ以外は数。
fn count_cell(count: u32) -> String {
    if count == 0_u32 {
        return String::from(NOTHING);
    }
    format!("{count}")
}

/// `BRANCH` 列。
fn branch_cell(local: &LocalReport) -> String {
    match *local {
        LocalReport::State(ref state) => match *state.head() {
            Head::Branch(ref name) => name.clone(),
            Head::Detached => String::from(DETACHED),
        },
        LocalReport::Unavailable => String::from(UNKNOWN),
    }
}

/// `DIRTY` 列。
fn dirty_cell(local: &LocalReport) -> String {
    match *local {
        LocalReport::State(ref state) => count_cell(state.dirty()),
        LocalReport::Unavailable => String::from(UNKNOWN),
    }
}

/// `AHEAD/BEHIND` 列。上流が無ければ `(none)`。
fn divergence_cell(local: &LocalReport) -> String {
    match *local {
        LocalReport::State(ref state) => match state.upstream() {
            Some(_) => divergence_text(state.ahead(), state.behind()),
            None => String::from(NO_UPSTREAM),
        },
        LocalReport::Unavailable => String::from(UNKNOWN),
    }
}

/// `+2/-1` の形。片方が 0 なら片方だけ、両方 0 なら `-`。
fn divergence_text(ahead: u32, behind: u32) -> String {
    match (ahead, behind) {
        (0_u32, 0_u32) => String::from(NOTHING),
        (_, 0_u32) => format!("+{ahead}"),
        (0_u32, _) => format!("-{behind}"),
        (_, _) => format!("+{ahead}/-{behind}"),
    }
}

/// `PR` 列。
fn pull_request_cell(remote: &RemoteReport) -> String {
    match *remote {
        RemoteReport::State(ref state) => count_cell(state.open_pull_requests()),
        RemoteReport::NotOnGithub => String::from(NOT_APPLICABLE),
        RemoteReport::Unavailable => String::from(UNKNOWN),
    }
}

/// `CI` 列。
fn ci_cell(remote: &RemoteReport) -> String {
    match *remote {
        RemoteReport::State(ref state) => String::from(match state.ci() {
            CiState::Success => "ok",
            CiState::Failure => "FAIL",
            CiState::Pending => "...",
            CiState::Absent => NOTHING,
        }),
        RemoteReport::NotOnGithub => String::from(NOT_APPLICABLE),
        RemoteReport::Unavailable => String::from(UNKNOWN),
    }
}

/// `STALE` 列。数え切れなかったら `?`。
fn stale_cell(remote: &RemoteReport, freshness: &Freshness) -> String {
    match *remote {
        RemoteReport::State(ref state) => match state.stale_branches(freshness) {
            StaleCount::Known(count) => count_cell(count),
            StaleCount::Truncated => String::from(UNKNOWN),
        },
        RemoteReport::NotOnGithub => String::from(NOT_APPLICABLE),
        RemoteReport::Unavailable => String::from(UNKNOWN),
    }
}

#[cfg(test)]
mod tests {
    use super::Row;
    use crate::branch_list::BranchList;
    use crate::ci_state::CiState;
    use crate::day::Day;
    use crate::freshness::Freshness;
    use crate::local_report::LocalReport;
    use crate::local_state::parse_porcelain;
    use crate::remote_branch::RemoteBranch;
    use crate::remote_report::RemoteReport;
    use crate::remote_state::RemoteState;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    fn freshness() -> Freshness {
        Freshness::new(Day::parse_iso8601("2026-09-02").expect("読める"), 30_u32)
    }

    fn local(source: &str) -> LocalReport {
        LocalReport::State(parse_porcelain(source).expect("読めるはずである"))
    }

    fn remote(pull_requests: u32, ci: CiState, truncated: bool) -> RemoteReport {
        let branches = BranchList::new(
            vec![RemoteBranch::new(
                String::from("old"),
                Day::parse_iso8601("2026-01-01").expect("読める"),
            )],
            truncated,
        );
        RemoteReport::State(RemoteState::new(
            Some(String::from("main")),
            ci,
            pull_requests,
            branches,
        ))
    }

    fn row(local: LocalReport, remote: RemoteReport) -> Row {
        Row::new(String::from("alpha"), local, remote)
    }

    fn cells(row: &Row) -> Vec<String> {
        row.cells(&freshness()).to_vec()
    }

    #[test]
    fn a_clean_repository_shows_dashes() {
        let source = "# branch.head main\n# branch.upstream origin/main\n# branch.ab +0 -0\n";
        let found = row(local(source), remote(0_u32, CiState::Success, false));
        assert_eq!(
            cells(&found),
            vec!["alpha", "main", "-", "-", "-", "ok", "1"]
        );
    }

    #[test]
    fn ahead_only_and_behind_only_are_written_alone() {
        let ahead = "# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -0\n";
        let behind = "# branch.head main\n# branch.upstream origin/main\n# branch.ab +0 -1\n";
        let both = "# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n";
        assert_eq!(
            cells(&row(local(ahead), RemoteReport::NotOnGithub)).get(3_usize),
            Some(&String::from("+2"))
        );
        assert_eq!(
            cells(&row(local(behind), RemoteReport::NotOnGithub)).get(3_usize),
            Some(&String::from("-1"))
        );
        assert_eq!(
            cells(&row(local(both), RemoteReport::NotOnGithub)).get(3_usize),
            Some(&String::from("+2/-1"))
        );
    }

    /// 上流が無いことと、差が無いことを分けて出す。
    #[test]
    fn a_branch_without_an_upstream_says_none() {
        let found = row(local("# branch.head main\n"), RemoteReport::NotOnGithub);
        assert_eq!(
            cells(&found),
            vec!["alpha", "main", "-", "(none)", "n/a", "n/a", "n/a"]
        );
    }

    #[test]
    fn a_detached_head_is_named() {
        let found = row(
            local("# branch.head (detached)\n? scratch/\n"),
            RemoteReport::Unavailable,
        );
        assert_eq!(
            cells(&found),
            vec!["alpha", "(detached)", "1", "(none)", "?", "?", "?"]
        );
    }

    #[test]
    fn the_ci_states_have_their_own_marks() {
        let source = "# branch.head main\n";
        let marks = [
            (CiState::Success, "ok"),
            (CiState::Failure, "FAIL"),
            (CiState::Pending, "..."),
            (CiState::Absent, "-"),
        ];
        for (state, mark) in marks {
            let found = row(local(source), remote(0_u32, state, false));
            assert_eq!(cells(&found).get(5_usize), Some(&String::from(mark)));
        }
    }

    /// 数え切れなかった枝は `?`。
    #[test]
    fn a_truncated_branch_list_shows_a_question_mark() {
        let found = row(
            local("# branch.head main\n"),
            remote(0_u32, CiState::Success, true),
        );
        assert_eq!(cells(&found).get(6_usize), Some(&String::from("?")));
        assert!(!found.is_complete(&freshness()));
    }

    #[test]
    fn a_row_is_complete_when_nothing_is_unknown() {
        let source = "# branch.head main\n";
        assert!(
            row(local(source), remote(0_u32, CiState::Success, false)).is_complete(&freshness())
        );
        assert!(row(local(source), RemoteReport::NotOnGithub).is_complete(&freshness()));
        assert!(!row(local(source), RemoteReport::Unavailable).is_complete(&freshness()));
        assert!(
            !row(LocalReport::Unavailable, RemoteReport::NotOnGithub).is_complete(&freshness())
        );
    }

    #[test]
    fn keeps_the_name_it_was_given() {
        let found = row(LocalReport::Unavailable, RemoteReport::Unavailable);
        assert_eq!(found.name(), "alpha");
    }
}
