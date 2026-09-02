//! 引数の読み方が分からなかったこと。

use core::fmt;

/// 使い方の1行。`--help` の usage と**同じ文字列を2箇所に書かない**。
pub(crate) const USAGE: &str = "fleet-top [DIR] [--stale-days N] [--no-github]";

/// 引数の読み方が分からなかったこと。
///
/// 🔑 枝を持つのは「使い方を出す」以外の対応が要るからではなく、
/// **直し方を言えるものは言う**ためである。`--stale-days` の値は人が手で打つので、
/// 「usage: …」だけを返しても、どこが悪いのか分からない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UsageError {
    /// 引数の形が違う（ディレクトリが2つ以上・知らない旗）。
    Arguments,
    /// `--stale-days` に値が無い。
    StaleDaysWithoutValue,
    /// `--stale-days` の値が日数として読めない。
    StaleDaysNotANumber(String),
    /// `--stale-days` が2回以上書かれた。**後勝ちにしない。**
    RepeatedStaleDays,
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Arguments => write!(f, "usage: {USAGE}"),
            Self::StaleDaysWithoutValue => f.write_str("--stale-days: 日数が続いていない"),
            Self::StaleDaysNotANumber(ref text) => {
                write!(f, "--stale-days: `{text}` は 0 以上の整数ではない")
            }
            Self::RepeatedStaleDays => {
                f.write_str("--stale-days: 2回以上書かれている（どちらが効くかを決めない）")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{USAGE, UsageError};

    /// 使い方の1行は usage の中に必ず現れる。
    #[test]
    fn the_generic_error_prints_the_usage_line() {
        assert_eq!(
            format!("{}", UsageError::Arguments),
            format!("usage: {USAGE}")
        );
    }

    /// 🔑 直し方を言えるものは言う。値の誤りは**打った文字**を返す。
    #[test]
    fn a_bad_value_repeats_what_was_typed() {
        let error = UsageError::StaleDaysNotANumber(String::from("x"));
        assert_eq!(
            format!("{error}"),
            "--stale-days: `x` は 0 以上の整数ではない"
        );
    }
}
