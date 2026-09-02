//! 実行の結果と、それが表す終了コード。

use std::process::ExitCode;

/// 実行の結果。
///
/// 🔴 **表が出ていても、`?` が1つでもあれば 1 で終わる**（設計メモ F-5）。
/// 「一部しか見ていない結果」を成功と呼ばないのが、この道具が生まれた事故
/// （片方だけ見て判断した）への答えである。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// 全行が確定した（`?` が無い）。
    Complete,
    /// `?` を含む行がある。表は出ている。
    Partial,
    /// 使い方の誤り・ディレクトリが読めない・時計が読めない。表は出ていない。
    Usage,
}

impl Outcome {
    /// 全行が確定しているかどうかから作る。
    pub(crate) fn of(complete: bool) -> Self {
        if complete {
            Self::Complete
        } else {
            Self::Partial
        }
    }

    /// 終了コード。
    pub(crate) fn code(self) -> ExitCode {
        match self {
            Self::Complete => ExitCode::SUCCESS,
            Self::Partial => ExitCode::from(1_u8),
            Self::Usage => ExitCode::from(2_u8),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Outcome;

    #[test]
    fn an_incomplete_table_is_not_a_success() {
        assert_eq!(Outcome::of(true), Outcome::Complete);
        assert_eq!(Outcome::of(false), Outcome::Partial);
        assert_ne!(Outcome::Partial, Outcome::Usage);
    }
}
