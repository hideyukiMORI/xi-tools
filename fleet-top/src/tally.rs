//! 1 回の実行で何を見たか（stderr の要約の中身）。

use crate::outcome::Outcome;

/// 1 回の実行で見たものの数と、結果。
///
/// 🔑 **数を stderr に出す。** 表は stdout だけに出すので、パイプで受けた側に
/// 「何リポジトリ見たか」は伝わらない。`fleet-top | grep FAIL` が空だったとき、
/// 「落ちている CI が無い」のか「1 つも見ていない」のかを区別できる必要がある。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tally {
    repositories: usize,
    on_github: usize,
    outcome: Outcome,
}

impl Tally {
    /// 見たリポジトリ数・GitHub に聞いた数・結果から作る。
    pub(crate) fn new(repositories: usize, on_github: usize, outcome: Outcome) -> Self {
        Self {
            repositories,
            on_github,
            outcome,
        }
    }

    /// 表に出したリポジトリの数。
    pub(crate) fn repositories(&self) -> usize {
        self.repositories
    }

    /// GitHub に**実際に聞いた**リポジトリの数。
    ///
    /// 🔴 `--no-github` のときは 0 である。origin が GitHub だったリポジトリの数ではない
    /// ——聞いていないのだから、数えられない。
    pub(crate) fn on_github(&self) -> usize {
        self.on_github
    }

    /// 全行が確定したかどうか。終了コードになる。
    pub(crate) fn outcome(&self) -> Outcome {
        self.outcome
    }
}
