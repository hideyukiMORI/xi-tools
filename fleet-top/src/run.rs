//! 走査 → ローカル（並列）→ GitHub（並列）→ 表 の流れ。
//!
//! 🔴 **局面は 2 つで、順に閉じる。** ローカルの結果（origin の owner/name）が
//! GitHub の入力になるので、全リポジトリの `git` が返ってから `gh` を投げる。
//! 局面の中は全部並列で、全体の壁時計は `gh` の往復で決まる（設計メモ F-3）。
//!
//! 🔴 **失敗した行を消さない。** 取れなかった値は `?` として表に残り、
//! 理由は stderr に 1 行ずつ出る。行を消すのは、この道具が生まれた事故
//! （片方だけ見て判断した）と同じ形である（設計メモ F-5）。

use std::io;

use fleet_top_core::day::Day;
use fleet_top_core::freshness::Freshness;
use fleet_top_core::github_slug::GithubSlug;
use fleet_top_core::local_report::LocalReport;
use fleet_top_core::remote_error::RemoteError;
use fleet_top_core::remote_report::RemoteReport;
use fleet_top_core::remote_state::RemoteState;
use fleet_top_core::row::Row;
use fleet_top_core::stale_count::StaleCount;
use fleet_top_core::table::render;

use crate::chunk_outcome::ChunkOutcome;
use crate::github;
use crate::github_access::GithubAccess;
use crate::local;
use crate::local_finding::LocalFinding;
use crate::options::Options;
use crate::outcome::Outcome;
use crate::output;
use crate::parallel;
use crate::repository::Repository;
use crate::scan;
use crate::tally::Tally;
use crate::target::Target;

/// 走査して表を出し、見た数と結果を返す。
///
/// # Errors
///
/// 走査するディレクトリが読めないときは [`io::Error`]。表は出ない。
pub(crate) fn report(options: &Options, today: Day) -> io::Result<Tally> {
    let repositories = scan::directory(options.directory())?;
    let freshness = Freshness::new(today, options.stale_days());

    let findings = examine(&repositories);
    let queried = asked(&findings, options.github());
    let remotes = enquire(&repositories, &findings, options.github(), &freshness);

    let rows = compose(&repositories, findings, remotes);
    output::table(&render(&rows, &freshness));

    let complete = rows.iter().all(|row| row.is_complete(&freshness));
    Ok(Tally::new(rows.len(), queried, Outcome::of(complete)))
}

/// ローカルの局面。全リポジトリに `git` を並列に打ち、理由を**走査順**に報告する。
fn examine(repositories: &[Repository]) -> Vec<LocalFinding> {
    let tasks: Vec<&Repository> = repositories.iter().collect();
    let findings = parallel::map(tasks, &local::inspect);
    for (repository, finding) in repositories.iter().zip(&findings) {
        if let Some(why) = finding.problem() {
            output::problem(&[repository.name()], why);
        }
    }
    findings
}

/// GitHub に**実際に聞いた**リポジトリの数。
fn asked(findings: &[LocalFinding], access: GithubAccess) -> usize {
    match access {
        GithubAccess::Skip => 0_usize,
        GithubAccess::Query => findings
            .iter()
            .filter(|finding| finding.slug().is_some())
            .count(),
    }
}

/// GitHub の局面。`--no-github` なら 1 度も起動しない。
fn enquire(
    repositories: &[Repository],
    findings: &[LocalFinding],
    access: GithubAccess,
    freshness: &Freshness,
) -> Vec<RemoteReport> {
    match access {
        GithubAccess::Skip => findings.iter().map(|_| RemoteReport::NotOnGithub).collect(),
        GithubAccess::Query => {
            let reports = query(repositories, findings);
            warn_truncated(repositories, &reports, freshness);
            reports
        }
    }
}

/// 枝が多すぎて STALE を数えられなかったリポジトリを報告する。
///
/// 🔴 **失敗ではないが `?` である。** `?` には必ず理由を添える（設計メモ F-5）。
/// 実際に 2026-09-02 の smoke で、枝 100 本超のリポジトリが理由の無い `?` を
/// 1 つ出し、終了コードだけが 1 になった。読む人は「何が読めなかったのか」を探すことになる。
fn warn_truncated(repositories: &[Repository], reports: &[RemoteReport], freshness: &Freshness) {
    for (repository, report) in repositories.iter().zip(reports) {
        let RemoteReport::State(state) = report else {
            continue;
        };
        if matches!(state.stale_branches(freshness), StaleCount::Truncated) {
            output::problem(
                &[repository.name()],
                "枝が 100 本を超えている。STALE は数えていない",
            );
        }
    }
}

/// 塊に割って並列に聞き、**塊の投入順**に結果を戻す。
fn query(repositories: &[Repository], findings: &[LocalFinding]) -> Vec<RemoteReport> {
    let mut reports: Vec<RemoteReport> = findings.iter().map(unasked).collect();
    let targets: Vec<Target> = findings
        .iter()
        .enumerate()
        .filter_map(|(index, finding)| finding.slug().cloned().map(|slug| Target::new(index, slug)))
        .collect();
    let chunks = github::chunk(&targets);
    let queries: Vec<Vec<GithubSlug>> = chunks
        .iter()
        .map(|chunk| chunk.iter().map(|target| target.slug().clone()).collect())
        .collect();

    let outcomes = parallel::map(queries, &|slugs: Vec<GithubSlug>| github::fetch(&slugs));
    for (chunk, outcome) in chunks.iter().zip(outcomes) {
        settle(repositories, chunk, outcome, &mut reports);
    }
    reports
}

/// 聞く前の状態。
///
/// 🔴 **手元が読めなかったリポジトリは `n/a` にしない。** origin を聞けていないので、
/// 「GitHub に無い」とは言えない。言えるのは「読めなかった」（`?`）だけである。
fn unasked(finding: &LocalFinding) -> RemoteReport {
    if finding.slug().is_some() {
        return RemoteReport::Unavailable;
    }
    match *finding.report() {
        LocalReport::State(_) => RemoteReport::NotOnGithub,
        LocalReport::Unavailable => RemoteReport::Unavailable,
    }
}

/// 1 塊の応答を、理由の報告と行への書き戻しに分けて片付ける。
fn settle(
    repositories: &[Repository],
    chunk: &[Target],
    outcome: ChunkOutcome,
    reports: &mut [RemoteReport],
) {
    let answers = answers(chunk.len(), outcome);
    tell(repositories, chunk, &answers);
    for (target, answer) in chunk.iter().zip(answers) {
        let Some(slot) = reports.get_mut(target.index()) else {
            continue;
        };
        *slot = match answer {
            Ok(state) => RemoteReport::State(state),
            Err(_) => RemoteReport::Unavailable,
        };
    }
}

/// 塊の結果を、リポジトリごとの「状態か、理由の文字列か」に均す。
fn answers(count: usize, outcome: ChunkOutcome) -> Vec<Result<RemoteState, String>> {
    match outcome {
        ChunkOutcome::Failed(shared) => (0_usize..count).map(|_| Err(shared.clone())).collect(),
        ChunkOutcome::Answered(results) => results
            .into_iter()
            .map(|result| result.map_err(|error| why(&error)))
            .collect(),
    }
}

/// 失敗の理由 1 行。
///
/// 🔑 [`RemoteError::Rejected`] は **GitHub が書いた原文**である（`Bad credentials` 等）。
/// `Display` の「GitHub が拒んだ: 」を前に付けると、GitHub の文がもう一度説明されるだけになる。
/// 他の 2 つは中身が位置や種別なので、`Display` の言い回しがそのまま理由になる。
fn why(error: &RemoteError) -> String {
    match *error {
        RemoteError::Rejected(ref message) => message.clone(),
        RemoteError::NotFound | RemoteError::Malformed(_) => error.to_string(),
    }
}

/// 理由を stderr に出す。**塊が丸ごと同じ理由なら 1 行にまとめる。**
fn tell(repositories: &[Repository], chunk: &[Target], answers: &[Result<RemoteState, String>]) {
    if let Some(shared) = common_reason(answers) {
        let names: Vec<&str> = chunk
            .iter()
            .filter_map(|target| name_of(repositories, target))
            .collect();
        output::problem(&names, shared);
        return;
    }
    for (target, answer) in chunk.iter().zip(answers) {
        let (Err(why), Some(name)) = (answer, name_of(repositories, target)) else {
            continue;
        };
        output::problem(&[name], why);
    }
}

/// 塊の全リポジトリが同じ理由で落ちたなら、その理由。
fn common_reason(answers: &[Result<RemoteState, String>]) -> Option<&str> {
    let first = answers.first()?.as_ref().err()?.as_str();
    answers
        .iter()
        .all(|answer| answer.as_ref().err().map(String::as_str) == Some(first))
        .then_some(first)
}

/// その塊の相手が、表の何行目のどのディレクトリだったか。
fn name_of<'a>(repositories: &'a [Repository], target: &Target) -> Option<&'a str> {
    repositories.get(target.index()).map(Repository::name)
}

/// 走査したリポジトリと 2 つの報告を、表の行にする。
///
/// 🔑 **リポジトリの数と行の数は必ず一致する。** 並べ替えは `render` が名前順に行う。
fn compose(
    repositories: &[Repository],
    findings: Vec<LocalFinding>,
    remotes: Vec<RemoteReport>,
) -> Vec<Row> {
    repositories
        .iter()
        .zip(findings)
        .zip(remotes)
        .map(|((repository, finding), remote)| {
            Row::new(
                String::from(repository.name()),
                finding.into_report(),
                remote,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{answers, common_reason, unasked, why};
    use crate::chunk_outcome::ChunkOutcome;
    use crate::local_finding::LocalFinding;
    use fleet_top_core::github_slug::GithubSlug;
    use fleet_top_core::local_report::LocalReport;
    use fleet_top_core::local_state::parse_porcelain;
    use fleet_top_core::remote_error::RemoteError;
    use fleet_top_core::remote_report::RemoteReport;

    fn state() -> LocalReport {
        LocalReport::State(parse_porcelain("# branch.head main\n").expect("読めるはず"))
    }

    fn slug() -> GithubSlug {
        GithubSlug::new("example-org", "alpha").expect("作れるはず")
    }

    /// 🔴 手元が読めなかった行の GitHub 列は `n/a` ではなく `?` である。
    #[test]
    fn an_unreadable_repository_is_never_reported_as_absent_from_github() {
        let broken = LocalFinding::new(LocalReport::Unavailable, None, Some(String::from("x")));
        assert_eq!(unasked(&broken), RemoteReport::Unavailable);

        let elsewhere = LocalFinding::new(state(), None, None);
        assert_eq!(unasked(&elsewhere), RemoteReport::NotOnGithub);

        let asked = LocalFinding::new(state(), Some(slug()), None);
        assert_eq!(unasked(&asked), RemoteReport::Unavailable);
    }

    /// 塊ごと落ちたら、リポジトリの数だけ同じ理由が並ぶ。
    #[test]
    fn a_failed_chunk_fails_every_repository_in_it() {
        let found = answers(3_usize, ChunkOutcome::Failed(String::from("gh が無い")));
        assert_eq!(found.len(), 3);
        assert_eq!(common_reason(&found), Some("gh が無い"));
    }

    /// 理由が揃っていなければ、まとめない。
    #[test]
    fn mixed_reasons_are_not_merged() {
        let found = answers(
            2_usize,
            ChunkOutcome::Answered(vec![
                Err(RemoteError::NotFound),
                Err(RemoteError::Rejected(String::from("Bad credentials"))),
            ]),
        );
        assert_eq!(common_reason(&found), None);
    }

    /// 成功が 1 つでも混ざれば、まとめない（空の塊も同じ）。
    #[test]
    fn a_chunk_without_failures_has_no_common_reason() {
        assert_eq!(common_reason(&[]), None);
    }

    /// 🔑 GitHub の原文をそのまま出す。`Display` の飾りを足さない。
    #[test]
    fn a_rejection_keeps_the_message_github_wrote() {
        assert_eq!(
            why(&RemoteError::Rejected(String::from("Bad credentials"))),
            "Bad credentials"
        );
        assert_eq!(why(&RemoteError::NotFound), "GitHub にそのリポジトリが無い");
        assert_eq!(
            why(&RemoteError::Malformed(String::from("r0.refs"))),
            "応答の形が想定と違う: r0.refs"
        );
    }
}
