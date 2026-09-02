//! `git` に聞いて分かったこと（1 リポジトリぶん）。

use fleet_top_core::github_slug::GithubSlug;
use fleet_top_core::local_report::LocalReport;

/// 1 リポジトリについて `git` に聞いた結果。
///
/// 🔑 **3 つを一緒に持つ。** 手元の状態・GitHub の owner/name・失敗の理由は
/// 同じ 2 回のサブプロセス起動から出てくるもので、別々に持ち回ると
/// 「`git status` は失敗したのに origin だけ取れている行」が作れてしまう。
/// 実際には [`LocalReport::Unavailable`] のとき `slug` は必ず `None` である
/// （`git` が動かないリポジトリに origin を聞きに行かない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalFinding {
    report: LocalReport,
    slug: Option<GithubSlug>,
    problem: Option<String>,
}

impl LocalFinding {
    /// 手元の状態・GitHub の owner/name・失敗の理由から作る。
    pub(crate) fn new(
        report: LocalReport,
        slug: Option<GithubSlug>,
        problem: Option<String>,
    ) -> Self {
        Self {
            report,
            slug,
            problem,
        }
    }

    /// GitHub の owner/name。origin が GitHub でなければ `None`。
    pub(crate) fn slug(&self) -> Option<&GithubSlug> {
        self.slug.as_ref()
    }

    /// 取れなかった理由。stderr に 1 行で出す。
    pub(crate) fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    /// 表に出す手元の状態（読むだけ）。
    pub(crate) fn report(&self) -> &LocalReport {
        &self.report
    }

    /// 表に出す手元の状態。
    pub(crate) fn into_report(self) -> LocalReport {
        self.report
    }
}
