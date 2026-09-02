//! GitHub 側の状態を取れなかったことの報告。

use alloc::string::String;
use core::error::Error;
use core::fmt;

/// 1 リポジトリぶんの GitHub の状態を取れなかった理由。
///
/// 🔑 **リクエスト全体ではなく 1 リポジトリぶんである。** `gh api graphql` は
/// `errors` が 1 件でもあると終了コード 1 を返すが、`data` には成功したリポジトリが
/// 入っている（設計メモの実測）。終了コードで全部捨てると、1 リポジトリの失敗で
/// 同じリクエストの他のリポジトリまで消える。**リポジトリごとに結果を持つ。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteError {
    /// GitHub にそのリポジトリが無い（`type` が `NOT_FOUND`）。
    NotFound,
    /// GitHub が拒んだ。**message は GitHub が書いた原文**である。
    Rejected(String),
    /// 応答の形が想定と違う。中身は読めなかった位置（`r1.pullRequests.totalCount` の形）。
    Malformed(String),
}

impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NotFound => f.write_str("repository not found on GitHub"),
            Self::Rejected(ref message) => write!(f, "GitHub rejected the request: {message}"),
            Self::Malformed(ref path) => write!(f, "unexpected response shape: {path}"),
        }
    }
}

impl Error for RemoteError {}

#[cfg(test)]
mod tests {
    use super::RemoteError;
    use alloc::format;
    use alloc::string::String;

    #[test]
    fn display_carries_the_message_and_the_path() {
        assert_eq!(
            format!("{}", RemoteError::Rejected(String::from("Bad credentials"))),
            "GitHub rejected the request: Bad credentials"
        );
        assert_eq!(
            format!(
                "{}",
                RemoteError::Malformed(String::from("r1.pullRequests.totalCount"))
            ),
            "unexpected response shape: r1.pullRequests.totalCount"
        );
        assert!(!format!("{}", RemoteError::NotFound).is_empty());
    }

    /// `core::error::Error` として扱える（bin 側で `dyn Error` に載せるため）。
    #[test]
    fn is_a_std_error() {
        let error = RemoteError::NotFound;
        let raised: &dyn core::error::Error = &error;
        assert!(!format!("{raised}").is_empty());
    }
}
