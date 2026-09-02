//! 既定枝の先頭コミットの CI の状態。

/// 既定枝の先頭コミットに付いた検査の総合結果（GraphQL の `statusCheckRollup.state`）。
///
/// 🔴 **`Absent`（検査そのものが無い）と `Failure` を混ぜない。** CI を設定していない
/// リポジトリと、CI が落ちているリポジトリは別の話である。表でも `-` と `FAIL` に分かれる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiState {
    /// 全部通った（`SUCCESS`）。
    Success,
    /// 落ちた（`FAILURE` / `ERROR`）。
    Failure,
    /// まだ動いている（`PENDING` / `EXPECTED`）。
    Pending,
    /// 検査が付いていない（`statusCheckRollup` が `null`、または既定枝が無い）。
    Absent,
}

impl CiState {
    /// GraphQL の `statusCheckRollup.state` の文字列を読む。
    ///
    /// 🔴 **知らない文字列を `Absent` に丸めない。** 丸めると、GitHub が状態を増やした日に
    /// 「CI が無いリポジトリ」が黙って増える。読めない文字列は読めないと言う（RS-002）。
    pub(crate) fn parse(text: &str) -> Option<Self> {
        match text {
            "SUCCESS" => Some(Self::Success),
            "FAILURE" | "ERROR" => Some(Self::Failure),
            "PENDING" | "EXPECTED" => Some(Self::Pending),
            // 🔑 `_` を書けるのは、これが enum ではなく文字列の照合だからである
            //    （`wildcard_enum_match_arm` の対象外）。閉じた集合は上の 5 語で、
            //    それ以外は「知らない状態」として呼び手に返す。
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CiState;

    #[test]
    fn reads_the_states_github_returns() {
        assert_eq!(CiState::parse("SUCCESS"), Some(CiState::Success));
        assert_eq!(CiState::parse("FAILURE"), Some(CiState::Failure));
        assert_eq!(CiState::parse("ERROR"), Some(CiState::Failure));
        assert_eq!(CiState::parse("PENDING"), Some(CiState::Pending));
        assert_eq!(CiState::parse("EXPECTED"), Some(CiState::Pending));
    }

    /// 知らない文字列は `Absent` に丸めず、読めないと言う。
    #[test]
    fn refuses_states_it_does_not_know() {
        assert_eq!(CiState::parse("SOMETHING_NEW"), None);
        assert_eq!(CiState::parse("success"), None);
        assert_eq!(CiState::parse(""), None);
    }
}
