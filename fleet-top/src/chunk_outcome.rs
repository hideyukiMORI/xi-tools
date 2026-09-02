//! 1 本の GraphQL リクエストの結果。

use fleet_top_core::remote_error::RemoteError;
use fleet_top_core::remote_state::RemoteState;

/// 1 塊（最大 [`fleet_top_core::graphql::REPOS_PER_QUERY`] リポジトリ）を聞いた結果。
///
/// 🔴 **「塊ごと失敗」と「リポジトリごとの結果」を分ける。** `gh` が起動できない・
/// 応答が JSON ですらない場合は塊の全リポジトリが同じ理由で落ちるので、
/// stderr に 1 行だけ出す。区別せずリポジトリ数ぶん同じ行を出すと、
/// `gh` が無い日の stderr が同じ文で 20 行埋まる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChunkOutcome {
    /// 塊ごと失敗した（`gh` が起動できない・stdout が JSON として読めない）。
    Failed(String),
    /// 応答を読めた。リポジトリごとに `Ok` / `Err` を持つ。
    Answered(Vec<Result<RemoteState, RemoteError>>),
}
