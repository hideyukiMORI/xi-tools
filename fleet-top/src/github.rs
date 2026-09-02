//! `gh api graphql` に GitHub の状態を聞く（1 塊 = 1 リクエスト）。
//!
//! 🔴 **終了コードで捨てない。** `gh api graphql` は `errors` が 1 件でもあると
//! 終了コード 1 を返すが、stdout の `data` には成功したリポジトリが入っている
//! （設計メモの実測）。捨てると、1 リポジトリの失敗で同じリクエストの他のリポジトリも消える。
//!
//! 🔑 token を扱わない。認証は `gh` から借りる（ADR 0003 決定 2）。

use std::process::Command;

use fleet_top_core::github_slug::GithubSlug;
use fleet_top_core::graphql::{REPOS_PER_QUERY, build_query, parse_response};
use fleet_top_core::json_parser::parse_json;

use crate::chunk_outcome::ChunkOutcome;
use crate::reason;
use crate::target::Target;

/// `gh` の出力が UTF-8 でなかったときの理由。
const NOT_UTF8: &str = "gh output is not UTF-8";

/// 聞く相手を [`REPOS_PER_QUERY`] 個ずつの塊に割る。
///
/// 🔑 **まとめるほど遅い。** 42 リポジトリを 1 本にすると 8.87 秒、60 リポジトリでは
/// HTTP 502 が返る。3 ずつに割って並列に投げると 60 リポジトリで 1.4 秒だった
/// （設計メモ「実測」）。割る数の正本は core の [`REPOS_PER_QUERY`] である。
pub(crate) fn chunk(targets: &[Target]) -> Vec<Vec<Target>> {
    targets
        .chunks(REPOS_PER_QUERY)
        .map(<[Target]>::to_vec)
        .collect()
}

/// 1 塊を `gh api graphql` に聞く。
///
/// 🔑 `-f query=<クエリ>` は**引数として**渡す（シェルを介さない）。クエリには
/// 改行と `{` `}` と引用符が入るので、シェルに解釈させると壊れる。
pub(crate) fn fetch(slugs: &[GithubSlug]) -> ChunkOutcome {
    let output = match Command::new("gh")
        .arg("api")
        .arg("graphql")
        .arg("-f")
        .arg(format!("query={}", build_query(slugs)))
        .output()
    {
        Ok(found) => found,
        // `gh` が入っていない（`ErrorKind::NotFound`）・起動できない。
        Err(error) => return ChunkOutcome::Failed(error.to_string()),
    };
    let Ok(text) = String::from_utf8(output.stdout) else {
        return ChunkOutcome::Failed(String::from(NOT_UTF8));
    };
    match parse_json(&text) {
        Ok(json) => ChunkOutcome::Answered(parse_response(&json, slugs)),
        // stdout が JSON ですらない（空・`gh` の使い方エラー）。理由は `gh` の言い分を優先する。
        Err(error) => ChunkOutcome::Failed(reason::first_line(&output.stderr, &error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::chunk;
    use crate::target::Target;
    use fleet_top_core::github_slug::GithubSlug;

    fn targets(count: usize) -> Vec<Target> {
        (0_usize..count)
            .map(|index| {
                let slug = GithubSlug::new("example-org", "alpha").expect("作れるはず");
                Target::new(index, slug)
            })
            .collect()
    }

    /// 🔑 7 個は 3 + 3 + 1 に割れる。最後の塊は短くてよい。
    #[test]
    fn seven_targets_become_three_chunks() {
        let found = chunk(&targets(7));
        let sizes: Vec<usize> = found.iter().map(Vec::len).collect();
        assert_eq!(sizes, [3_usize, 3_usize, 1_usize]);
    }

    /// 🔴 0 個なら 0 塊。`gh` を1回も起動しない。
    #[test]
    fn no_target_means_no_request() {
        assert!(chunk(&targets(0)).is_empty());
    }

    #[test]
    fn a_chunk_keeps_the_row_it_came_from() {
        let found = chunk(&targets(4));
        let last = found.last().and_then(|chunk| chunk.first());
        assert_eq!(last.map(Target::index), Some(3_usize));
    }
}
