//! 探すもの・探す場所・出し方。

use std::path::PathBuf;

use scopegrep_core::search_scope::SearchScope;

use crate::output_format::OutputFormat;

/// 1回の検索を決める全て。**環境から読んだ値はここまでで止まる**（RS-015）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    needle: String,
    paths: Vec<PathBuf>,
    format: OutputFormat,
    scope: SearchScope,
}

impl Options {
    /// 探す文字列・探すパス・出し方・探す範囲から作る。
    pub(crate) fn new(
        needle: String,
        paths: Vec<PathBuf>,
        format: OutputFormat,
        scope: SearchScope,
    ) -> Self {
        Self {
            needle,
            paths,
            format,
            scope,
        }
    }

    /// 探す固定文字列（正規表現ではない）。
    pub(crate) fn needle(&self) -> &str {
        &self.needle
    }

    /// 探す場所。与えられた順に見る。
    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// 出力の形。
    pub(crate) fn format(&self) -> OutputFormat {
        self.format
    }

    /// 探す範囲。既定は値だけで、`--comments` でコメントも探す。
    pub(crate) fn scope(&self) -> SearchScope {
        self.scope
    }
}
