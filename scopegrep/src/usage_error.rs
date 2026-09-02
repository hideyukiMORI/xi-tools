//! 引数の読み方が分からなかったこと。

use core::fmt;

use scopegrep_core::scope_pattern_error::ScopePatternError;

/// 使い方の1行。`--help` の usage と**同じ文字列を2箇所に書かない**。
pub(crate) const USAGE: &str =
    "scopegrep [-i] [--json] [--comments] [--scope <pattern>] <needle> [<path>...]";

/// 引数の読み方が分からなかったこと。
///
/// 🔑 枝を持つのは「使い方を出す」以外の対応が要るからではなく、
/// **直し方を言えるものは言う**ためである。`--scope` のパターンは人が手で打つので、
/// 「usage: …」だけを返しても、どこが悪いのか分からない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UsageError {
    /// 引数の形が違う（needle が無い・知らない旗）。
    Arguments,
    /// `--scope` のパターンが読めない。
    Scope(ScopePatternError),
    /// `--scope` に値が無い。
    ScopeWithoutPattern,
    /// `--scope` が2回以上書かれた。**後勝ちにしない。**
    RepeatedScope,
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Arguments => write!(f, "usage: {USAGE}"),
            Self::Scope(reason) => write!(f, "--scope: {reason}"),
            Self::ScopeWithoutPattern => f.write_str("--scope: パターンが続いていない"),
            Self::RepeatedScope => {
                f.write_str("--scope: 2回以上書かれている（どちらが効くかを決めない）")
            }
        }
    }
}
