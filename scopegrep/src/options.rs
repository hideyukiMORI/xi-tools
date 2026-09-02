//! 探すもの・探す場所・出し方。

use std::path::PathBuf;

use scopegrep_core::query::Query;

use crate::output_format::OutputFormat;

/// 1回の検索を決める全て。**環境から読んだ値はここまでで止まる**（RS-015）。
///
/// 🔑 「何を・どう探すか」は [`Query`] が持ち、ここが持つのは
/// **std に属するもの**（探す場所と出し方）だけである。境界がそのまま
/// `no_std` の中核とバイナリの境界になっている。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    query: Query,
    paths: Vec<PathBuf>,
    format: OutputFormat,
}

impl Options {
    /// 検索条件・探すパス・出し方から作る。
    pub(crate) fn new(query: Query, paths: Vec<PathBuf>, format: OutputFormat) -> Self {
        Self {
            query,
            paths,
            format,
        }
    }

    /// 何を・どう探すか。
    pub(crate) fn query(&self) -> &Query {
        &self.query
    }

    /// 探す場所。与えられた順に見る。
    ///
    /// 🔑 **空のパスは「今いる場所」を指す**（引数を省略したとき）。
    /// `.` と区別するのは、表示に `./` を付けるかどうかが変わるからである。
    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// 出力の形。
    pub(crate) fn format(&self) -> OutputFormat {
        self.format
    }
}
