//! 走査する場所・古さの基準・GitHub に聞くかどうか。

use std::path::{Path, PathBuf};

use crate::github_access::GithubAccess;

/// 1回の実行を決める全て。**環境から読んだ値はここまでで止まる**（RS-015）。
///
/// 🔑 「今日」はここに入らない。時計を読むのは配線点（`main`）の仕事で、
/// 値は [`fleet_top_core::freshness::Freshness`] にして中核へ渡る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    directory: PathBuf,
    stale_days: u32,
    github: GithubAccess,
}

impl Options {
    /// 走査するディレクトリ・古さの基準・GitHub の扱いから作る。
    pub(crate) fn new(directory: PathBuf, stale_days: u32, github: GithubAccess) -> Self {
        Self {
            directory,
            stale_days,
            github,
        }
    }

    /// 走査するディレクトリ。**直下しか見ない**（再帰しない・設計メモ F-4）。
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    /// 何日で「古い枝」と呼ぶか（`--stale-days`。既定 30）。
    pub(crate) fn stale_days(&self) -> u32 {
        self.stale_days
    }

    /// GitHub に問い合わせるかどうか。
    pub(crate) fn github(&self) -> GithubAccess {
        self.github
    }
}
