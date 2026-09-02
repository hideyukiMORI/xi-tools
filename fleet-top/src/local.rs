//! `git` に手元の状態を聞く（1 リポジトリにつきサブプロセス 2 つ）。
//!
//! 🔴 **`git fetch` を打たない。見るだけである**（設計メモ「非目標」）。
//! `AHEAD/BEHIND` は手元の追跡枝との差で、リモートを取りに行った値ではない。

use std::io;
use std::path::Path;
use std::process::{Command, Output};

use fleet_top_core::github_slug::{GithubSlug, parse_remote_url};
use fleet_top_core::local_report::LocalReport;
use fleet_top_core::local_state::{LocalState, parse_porcelain};

use crate::local_finding::LocalFinding;
use crate::reason;
use crate::repository::Repository;

/// `git status` の出力が UTF-8 でなかったときの理由。
const STATUS_NOT_UTF8: &str = "git status の出力が UTF-8 ではない";

/// `git` が終了コードだけで失敗し、stderr に何も言わなかったときの理由。
const STATUS_FAILED: &str = "git status が失敗した（理由の出力が無い）";

/// 1 リポジトリの手元の状態と、GitHub の owner/name を集める。
///
/// 🔴 **`git status` が失敗したら origin を聞かない。** 失敗したリポジトリの
/// GitHub 列は `n/a`（GitHub に無い）ではなく `?`（読めなかった）である。
/// origin だけ読めても、それは「聞けなかった」ことを変えない。
pub(crate) fn inspect(repository: &Repository) -> LocalFinding {
    match status(repository.path()) {
        Ok(state) => LocalFinding::new(LocalReport::State(state), slug(repository.path()), None),
        Err(problem) => LocalFinding::new(LocalReport::Unavailable, None, Some(problem)),
    }
}

/// `git status --porcelain=v2 --branch` を読む。失敗の理由は文字列で返す。
fn status(path: &Path) -> Result<LocalState, String> {
    let output =
        git(path, &["status", "--porcelain=v2", "--branch"]).map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(reason::first_line(&output.stderr, STATUS_FAILED));
    }
    let text = String::from_utf8(output.stdout)
        .ok()
        .ok_or_else(|| String::from(STATUS_NOT_UTF8))?;
    parse_porcelain(&text).map_err(|error| error.to_string())
}

/// `git remote get-url origin` から GitHub の owner/name を読む。
///
/// 🔑 **失敗を理由として報告しない。** origin が無いリポジトリは珍しくなく、
/// それは「GitHub に置いていない」という**答え**である（表では `n/a`）。
/// エラー扱いにすると、置いていないだけのリポジトリが毎回 stderr を汚す。
fn slug(path: &Path) -> Option<GithubSlug> {
    let output = git(path, &["remote", "get-url", "origin"]).ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    parse_remote_url(text.trim())
}

/// `git -C <path> …` を起動する。
///
/// 🔴 **`GIT_OPTIONAL_LOCKS=0` を付ける。** 並列に `status` を打つと、
/// git が index を書き戻そうとして `index.lock` を取り合う。ここは読むだけなので、
/// その書き戻しは要らない。
fn git(path: &Path, arguments: &[&str]) -> io::Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(arguments)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
}

#[cfg(test)]
mod tests {
    use super::inspect;
    use crate::repository::Repository;
    use fleet_top_core::local_report::LocalReport;
    use std::fs;
    use std::path::PathBuf;

    fn temporary(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fleet-top-local-{}-{name}", std::process::id()))
    }

    /// 🔴 `.git` があっても中身が壊れていれば `git` は失敗する。
    /// そのとき行は消えず、理由が付いて `Unavailable` になる。
    #[test]
    fn a_broken_repository_reports_why() {
        let root = temporary("broken");
        fs::create_dir_all(root.join(".git")).expect("作れるはず");
        let repository = Repository::new(String::from("broken"), root.clone());

        let found = inspect(&repository);
        fs::remove_dir_all(&root).expect("片付けられるはず");

        assert_eq!(found.slug(), None);
        assert!(found.problem().is_some(), "理由が付いていない");
        assert_eq!(found.into_report(), LocalReport::Unavailable);
    }
}
