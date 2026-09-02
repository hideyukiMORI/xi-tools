//! 走査の結果と、それが表す終了コード。

use std::process::ExitCode;

/// 走査の結果。**`grep` と同じ3値**である（設計メモ「終了コード」）。
///
/// 🔴 [`Outcome::Failed`] は他の全てに勝つ。読めなかったファイルがあったなら、
/// ヒットが出ていても 2 で終わる。「一部しか見ていない結果」を成功と呼ばない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// 1件以上ヒットした。
    Found,
    /// ヒットが無かった。
    Missing,
    /// エラーがあった。
    Failed,
}

impl Outcome {
    /// 2つの結果をまとめる。**エラーは吸収されない**。
    pub(crate) fn combine(self, other: Self) -> Self {
        match self {
            Self::Failed => Self::Failed,
            Self::Found => match other {
                Self::Failed => Self::Failed,
                Self::Found | Self::Missing => Self::Found,
            },
            Self::Missing => other,
        }
    }

    /// 終了コード。`grep` に合わせる（0 = あり / 1 = なし / 2 = エラー）。
    pub(crate) fn code(self) -> ExitCode {
        match self {
            Self::Found => ExitCode::SUCCESS,
            Self::Missing => ExitCode::from(1_u8),
            Self::Failed => ExitCode::from(2_u8),
        }
    }
}
