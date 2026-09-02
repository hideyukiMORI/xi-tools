//! 引数の読み方が分からなかったこと。

use core::fmt;

use scopegrep_core::scope_pattern_error::ScopePatternError;

/// 使い方の1行。`--help` の usage と**同じ文字列を2箇所に書かない**。
///
/// 🔑 `(<needle> | -e <regex>)` は**どちらか一方**である。`-e` を付けたときの
/// 位置引数はすべてパスになり、needle の位置は無くなる。
pub(crate) const USAGE: &str = "scopegrep [-i] [--json] [--comments] [--scope <pattern>] \
     (<needle> | -e <regex>) [<path>...]";

/// 引数の読み方が分からなかったこと。
///
/// 🔑 枝を持つのは「使い方を出す」以外の対応が要るからではなく、
/// **直し方を言えるものは言う**ためである。`--scope` のパターンは人が手で打つので、
/// 「usage: …」だけを返しても、どこが悪いのか分からない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UsageError {
    /// 引数の形が違う（needle が無い・知らない旗）。
    Arguments,
    /// `--scope` のパターンが読めない。
    Scope(ScopePatternError),
    /// `--scope` に値が無い。
    ScopeWithoutPattern,
    /// `--scope` が2回以上書かれた。**後勝ちにしない。**
    RepeatedScope,
    /// `-e` に値が無い。
    RegexWithoutPattern,
    /// `-e` が2回以上書かれた。**後勝ちにしない。**
    RepeatedRegex,
    /// 正規表現が読めない。理由は `regex` の言い分をそのまま渡す。
    #[cfg(feature = "regex")]
    Regex(String),
    /// 🔴 この binary は正規表現なしでビルドされている。
    /// **黙って固定文字列として扱わない**（ADR 0002 決定 3）。
    #[cfg(not(feature = "regex"))]
    RegexUnsupported,
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arguments => write!(f, "usage: {USAGE}"),
            Self::Scope(reason) => write!(f, "--scope: {reason}"),
            Self::ScopeWithoutPattern => f.write_str("--scope: パターンが続いていない"),
            Self::RepeatedScope => {
                f.write_str("--scope: 2回以上書かれている（どちらが効くかを決めない）")
            }
            Self::RegexWithoutPattern => f.write_str("-e: 正規表現が続いていない"),
            Self::RepeatedRegex => {
                f.write_str("-e: 2回以上書かれている（どちらが効くかを決めない）")
            }
            #[cfg(feature = "regex")]
            Self::Regex(reason) => write!(f, "正規表現が不正: {reason}"),
            #[cfg(not(feature = "regex"))]
            Self::RegexUnsupported => f.write_str(
                "この binary は正規表現なしでビルドされている（cargo install --features regex）",
            ),
        }
    }
}
