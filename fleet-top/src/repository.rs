//! 走査で見つかった git リポジトリ1つ。

use std::path::{Path, PathBuf};

/// ディレクトリ直下で見つかった git リポジトリ1つ。
///
/// 🔑 名前とパスを**別々に持つ**。表に出るのはディレクトリ名（`to_string_lossy` した
/// 表示用の文字列）で、`git -C` に渡すのは [`PathBuf`] である。名前からパスを組み直すと、
/// UTF-8 でないファイル名を持つディレクトリで別の場所を指す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Repository {
    name: String,
    path: PathBuf,
}

impl Repository {
    /// 表示名とパスから作る。
    pub(crate) fn new(name: String, path: PathBuf) -> Self {
        Self { name, path }
    }

    /// 表に出るディレクトリ名。並び順もこれで決まる（バイト順）。
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// `git -C` に渡すパス。
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}
