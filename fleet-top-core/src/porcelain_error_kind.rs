//! `git status --porcelain=v2` を読めなかった理由の種別。

use core::fmt;

/// porcelain v2 の出力を読めなかった理由。
///
/// 🔑 porcelain v2 は「機械が読むための形」として git が版を跨いで保つと約束した書式である。
/// それでも**知らない行が来たら黙って捨てない**（RS-002）。捨てると、読めていないのに
/// 「変更なし」に見える行が表に出る。この道具が生まれた事故と同じ形になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PorcelainErrorKind {
    /// `# branch.head` の行が無い（`--branch` を付けずに実行した出力）。
    MissingHead,
    /// `#` 見出しの値が読めない（`# branch.ab +x -1` のような形）。
    MalformedHeader,
    /// 見出しでもエントリでもない行が現れた。
    UnexpectedLine,
}

impl fmt::Display for PorcelainErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::MissingHead => {
                f.write_str("`# branch.head` が無い（`--branch` を付けて実行する）")
            }
            Self::MalformedHeader => f.write_str("`#` 見出しの値が読めない"),
            Self::UnexpectedLine => f.write_str("porcelain v2 の行として読めない"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PorcelainErrorKind;
    use alloc::format;

    /// 全ての種別が空でない説明を持つ。**説明の無い種別を足せない**ようにする。
    #[test]
    fn every_kind_has_a_message() {
        let kinds = [
            PorcelainErrorKind::MissingHead,
            PorcelainErrorKind::MalformedHeader,
            PorcelainErrorKind::UnexpectedLine,
        ];
        for kind in kinds {
            assert!(!format!("{kind}").is_empty());
        }
    }

    #[test]
    fn kinds_are_distinguishable() {
        assert_ne!(
            PorcelainErrorKind::MissingHead,
            PorcelainErrorKind::UnexpectedLine
        );
    }
}
