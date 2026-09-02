//! 引数を読んだ結果、この起動が何をするか。

use crate::options::Options;

/// この起動が何をするか。**引数の解析は「どれか一つ」に必ず落ちる**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Invocation {
    /// 表を出す。
    Report(Options),
    /// 使い方を出す。
    Help,
    /// 版を出す。
    Version,
}
