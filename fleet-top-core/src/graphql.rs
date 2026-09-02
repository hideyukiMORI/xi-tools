//! GitHub GraphQL のクエリの組み立てと、応答の読み取り。
//!
//! 🔑 **1 本のリクエストに複数のリポジトリをエイリアスで並べる。** 実測では
//! 1 本にまとめるほど遅くなり（42 リポで 8.87 秒、60 リポで HTTP 502）、
//! **3 リポずつに割って並列に投げる**のが最も速かった（60 リポで 1.4 秒）。
//! その「3」が [`REPOS_PER_QUERY`] である（設計メモ「実測」と ADR 0003）。
//!
//! ⚠️ `gh api graphql` は `errors` が 1 件でもあると終了コード 1 を返すが、
//! stdout の `data` には成功したリポジトリが入っている。**終了コードで捨てない。**
//! [`parse_response`] はリポジトリごとに `Ok` / `Err` を返す。

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::branch_list::BranchList;
use crate::ci_state::CiState;
use crate::day::Day;
use crate::github_slug::GithubSlug;
use crate::json_number::JsonNumber;
use crate::json_value::JsonValue;
use crate::remote_branch::RemoteBranch;
use crate::remote_error::RemoteError;
use crate::remote_state::RemoteState;

/// 1 本のクエリに載せるリポジトリの数。
///
/// 実測で決めた値である（設計メモ「実測」）。大きくすると GitHub 側が直列に解決して遅くなり、
/// 小さくするとリクエスト数が増える。60 リポジトリを 3 ずつ 20 本並列で 1.4 秒。
pub const REPOS_PER_QUERY: usize = 3;

/// 取りたいフィールド。fragment にして 1 回だけ書く（リポジトリごとに繰り返さない）。
const FRAGMENT: &str = "fragment RepoFields on Repository { nameWithOwner defaultBranchRef { name target { ... on Commit { committedDate statusCheckRollup { state } } } } pullRequests(states: OPEN) { totalCount } refs(refPrefix: \"refs/heads/\", first: 100) { totalCount nodes { name target { ... on Commit { committedDate } } } } }";

/// 応答のうち、成功したリポジトリが入る場所。
const DATA: &str = "data";
/// 応答のうち、失敗したリポジトリが入る場所。
const ERRORS: &str = "errors";
/// リクエスト全体が失敗したときの文言。
const MESSAGE: &str = "message";
/// そのリポジトリが無いことを表す `errors[].type`。
const NOT_FOUND: &str = "NOT_FOUND";

/// 既定枝への参照。
const DEFAULT_BRANCH_REF: &str = "defaultBranchRef";
/// 既定枝の先頭コミット。
const TARGET: &str = "defaultBranchRef.target";
/// 既定枝の先頭コミットに付いた検査の総合結果。
const ROLLUP: &str = "defaultBranchRef.target.statusCheckRollup";
/// その状態。
const ROLLUP_STATE: &str = "defaultBranchRef.target.statusCheckRollup.state";
/// open な PR。
const PULL_REQUESTS: &str = "pullRequests";
/// リモート枝。
const REFS: &str = "refs";
/// 件数のフィールド名。
const TOTAL_COUNT: &str = "totalCount";

/// エイリアス（`r0` `r1` …）を並べたクエリを組み立てる。
///
/// エイリアスは `slugs` の並びと同じ順で `r0` から振る。応答は
/// [`parse_response`] に同じ `slugs` を渡して読む。
///
/// 🔑 `owner` / `name` は [`GithubSlug`] が `[A-Za-z0-9_.-]` に限っているので、
/// **クエリに埋めても壊れない**（引用符も `\` も入らない）。エスケープが要らないのは
/// 型がそれを保証しているからである（RS-001）。
#[must_use]
pub fn build_query(slugs: &[GithubSlug]) -> String {
    let mut query = String::from("query {\n");
    for (index, slug) in slugs.iter().enumerate() {
        let line = alias_line(index, slug);
        query.push_str(&line);
    }
    query.push_str("}\n");
    query.push_str(FRAGMENT);
    query.push('\n');
    query
}

/// エイリアス 1 つぶんの行。
fn alias_line(index: usize, slug: &GithubSlug) -> String {
    format!(
        "  r{index}: repository(owner: \"{}\", name: \"{}\") {{ ...RepoFields }}\n",
        slug.owner(),
        slug.name()
    )
}

/// 応答を読んで、`slugs` と**同じ数・同じ順**の結果を返す。
///
/// 🔴 **失敗したリポジトリの行を消さない。** 1 リポジトリの失敗は
/// [`RemoteError`] として同じ位置に残り、表では `?` になる（設計メモ F-5）。
///
/// `data` が無い（`{"message":"Bad credentials"}` の形）ときは、リクエスト全体が
/// 失敗しているので全要素が [`RemoteError::Rejected`] になる。
#[must_use]
pub fn parse_response(
    json: &JsonValue,
    slugs: &[GithubSlug],
) -> Vec<Result<RemoteState, RemoteError>> {
    let Some(repositories) = json.get(DATA).filter(|value| value.as_object().is_some()) else {
        return rejected_all(json, slugs.len());
    };
    (0_usize..slugs.len())
        .map(|index| read_alias(repositories, json, index))
        .collect()
}

/// リクエスト全体が失敗したときの結果を、リポジトリの数だけ作る。
fn rejected_all(json: &JsonValue, count: usize) -> Vec<Result<RemoteState, RemoteError>> {
    let message = json
        .get(MESSAGE)
        .and_then(JsonValue::as_str)
        .or_else(|| first_error_message(json));
    let error = match message {
        Some(text) => RemoteError::Rejected(String::from(text)),
        None => RemoteError::Malformed(String::from(DATA)),
    };
    vec![Err(error); count]
}

/// `errors[0].message`。GraphQL が全体の失敗を `errors` で返す形に備える。
fn first_error_message(json: &JsonValue) -> Option<&str> {
    json.get(ERRORS)
        .and_then(JsonValue::as_array)?
        .first()?
        .get(MESSAGE)
        .and_then(JsonValue::as_str)
}

/// エイリアス 1 つぶんを読む。
fn read_alias(
    repositories: &JsonValue,
    json: &JsonValue,
    index: usize,
) -> Result<RemoteState, RemoteError> {
    let alias = format!("r{index}");
    let Some(value) = repositories.get(&alias) else {
        return Err(RemoteError::Malformed(alias));
    };
    if value.is_null() {
        return Err(error_for(json, &alias));
    }
    if value.as_object().is_none() {
        return Err(RemoteError::Malformed(alias));
    }
    read_state(&alias, value)
}

/// `data.rN` が `null` のとき、`errors` からそのリポジトリの失敗を探す。
fn error_for(json: &JsonValue, alias: &str) -> RemoteError {
    let entry = json
        .get(ERRORS)
        .and_then(JsonValue::as_array)
        .unwrap_or(&[])
        .iter()
        .find(|entry| points_at(entry, alias));
    match entry {
        Some(found) => classify(found),
        // `null` なのに理由が無い。応答として辻褄が合っていない。
        None => RemoteError::Malformed(String::from(alias)),
    }
}

/// `errors[].path` がちょうどこのエイリアスを指しているか。
fn points_at(entry: &JsonValue, alias: &str) -> bool {
    entry
        .get("path")
        .and_then(JsonValue::as_array)
        .is_some_and(|path| {
            path.len() == 1_usize && path.first().and_then(JsonValue::as_str) == Some(alias)
        })
}

/// `errors[]` の 1 件を [`RemoteError`] に写す。
fn classify(entry: &JsonValue) -> RemoteError {
    if entry.get("type").and_then(JsonValue::as_str) == Some(NOT_FOUND) {
        return RemoteError::NotFound;
    }
    match entry.get(MESSAGE).and_then(JsonValue::as_str) {
        Some(message) => RemoteError::Rejected(String::from(message)),
        None => RemoteError::Malformed(String::from("errors[].message")),
    }
}

/// 取れたリポジトリ 1 つぶんを読む。
fn read_state(alias: &str, repository: &JsonValue) -> Result<RemoteState, RemoteError> {
    Ok(RemoteState::new(
        read_default_branch(alias, repository)?,
        read_ci(alias, repository)?,
        read_pull_requests(alias, repository)?,
        read_branches(alias, repository)?,
    ))
}

/// 読めなかった位置を持つ [`RemoteError::Malformed`] を作る。
fn malformed(alias: &str, path: &str) -> RemoteError {
    RemoteError::Malformed(format!("{alias}.{path}"))
}

/// `defaultBranchRef`。**`null` は「空のリポジトリ」**であって、読めなかったのではない。
fn branch_ref<'a>(
    alias: &str,
    repository: &'a JsonValue,
) -> Result<Option<&'a JsonValue>, RemoteError> {
    let value = repository
        .get(DEFAULT_BRANCH_REF)
        .ok_or_else(|| malformed(alias, DEFAULT_BRANCH_REF))?;
    if value.is_null() {
        return Ok(None);
    }
    if value.as_object().is_none() {
        return Err(malformed(alias, DEFAULT_BRANCH_REF));
    }
    Ok(Some(value))
}

/// 既定枝の名前。
fn read_default_branch(alias: &str, repository: &JsonValue) -> Result<Option<String>, RemoteError> {
    let Some(reference) = branch_ref(alias, repository)? else {
        return Ok(None);
    };
    let name = reference
        .get("name")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| malformed(alias, "defaultBranchRef.name"))?;
    Ok(Some(String::from(name)))
}

/// 既定枝の先頭コミットの CI の状態。
fn read_ci(alias: &str, repository: &JsonValue) -> Result<CiState, RemoteError> {
    let Some(reference) = branch_ref(alias, repository)? else {
        return Ok(CiState::Absent);
    };
    let target = reference
        .get("target")
        .filter(|value| value.as_object().is_some())
        .ok_or_else(|| malformed(alias, TARGET))?;
    let rollup = target
        .get("statusCheckRollup")
        .ok_or_else(|| malformed(alias, ROLLUP))?;
    if rollup.is_null() {
        return Ok(CiState::Absent);
    }
    let state = rollup
        .get("state")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| malformed(alias, ROLLUP_STATE))?;
    CiState::parse(state).ok_or_else(|| malformed(alias, ROLLUP_STATE))
}

/// open な PR の数。
fn read_pull_requests(alias: &str, repository: &JsonValue) -> Result<u32, RemoteError> {
    read_total_count(alias, repository.get(PULL_REQUESTS), PULL_REQUESTS)
}

/// `<field>.totalCount` を `u32` として読む。
fn read_total_count(
    alias: &str,
    container: Option<&JsonValue>,
    field: &str,
) -> Result<u32, RemoteError> {
    let path = format!("{field}.{TOTAL_COUNT}");
    let count = container
        .and_then(|value| value.get(TOTAL_COUNT))
        .and_then(JsonValue::as_number)
        .and_then(JsonNumber::as_u64)
        .ok_or_else(|| malformed(alias, &path))?;
    // 🔴 `map_err(|_| ...)` で理由を捨てない（`map_err_ignore` が deny）。
    //    ここでは「収まらなかった」ことしか言えないので、`ok()` で落として位置を足す。
    u32::try_from(count)
        .ok()
        .ok_or_else(|| malformed(alias, &path))
}

/// リモート枝の一覧。`refs.totalCount` が取れた本数より多ければ切り詰められている。
fn read_branches(alias: &str, repository: &JsonValue) -> Result<BranchList, RemoteError> {
    let refs = repository
        .get(REFS)
        .filter(|value| value.as_object().is_some())
        .ok_or_else(|| malformed(alias, REFS))?;
    let total = read_total_count(alias, Some(refs), REFS)?;
    let nodes = refs
        .get("nodes")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| malformed(alias, "refs.nodes"))?;
    let branches = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| read_branch(alias, node, index))
        .collect::<Result<Vec<RemoteBranch>, RemoteError>>()?;
    let taken = u32::try_from(nodes.len()).unwrap_or(u32::MAX);
    Ok(BranchList::new(branches, total > taken))
}

/// リモート枝 1 本。
fn read_branch(alias: &str, node: &JsonValue, index: usize) -> Result<RemoteBranch, RemoteError> {
    let name = node
        .get("name")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| malformed(alias, &format!("refs.nodes[{index}].name")))?;
    let committed = node
        .get("target")
        .and_then(|target| target.get("committedDate"))
        .and_then(JsonValue::as_str)
        .and_then(Day::parse_iso8601)
        .ok_or_else(|| malformed(alias, &format!("refs.nodes[{index}].target.committedDate")))?;
    Ok(RemoteBranch::new(String::from(name), committed))
}

#[cfg(test)]
mod tests {
    use super::{REPOS_PER_QUERY, build_query, parse_response};
    use crate::ci_state::CiState;
    use crate::day::Day;
    use crate::freshness::Freshness;
    use crate::github_slug::GithubSlug;
    use crate::json_parser;
    use crate::json_value::JsonValue;
    use crate::remote_error::RemoteError;
    use crate::remote_state::RemoteState;
    use crate::stale_count::StaleCount;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    /// 前半で置いた架空の応答（実データではない・地雷 5）。
    const RESPONSE: &str = include_str!("../testdata/graphql-response.json");

    fn slugs(names: &[&str]) -> Vec<GithubSlug> {
        names
            .iter()
            .map(|name| GithubSlug::new("example-org", name).expect("受けるはずである"))
            .collect()
    }

    fn read(source: &str) -> JsonValue {
        json_parser::parse_json(source).expect("読めるはずである")
    }

    fn results(source: &str, count: usize) -> Vec<Result<RemoteState, RemoteError>> {
        let names: Vec<&str> = vec!["alpha", "beta", "gamma"];
        let taken = names.get(0_usize..count).unwrap_or(&names);
        parse_response(&read(source), &slugs(taken))
    }

    fn state(source: &str) -> RemoteState {
        results(source, 1_usize)
            .into_iter()
            .next()
            .expect("1 件返るはずである")
            .expect("読めるはずである")
    }

    fn error(source: &str) -> RemoteError {
        results(source, 1_usize)
            .into_iter()
            .next()
            .expect("1 件返るはずである")
            .expect_err("読めないはずである")
    }

    /// `r0` 1 つぶんの応答を組み立てる。差し替えたい部分だけを渡す。
    fn one(body: &str) -> String {
        alloc::format!("{{\"data\": {{\"r0\": {body}}}}}")
    }

    /// 既定の形（全部そろっている）。
    const WHOLE: &str = "{\"nameWithOwner\": \"example-org/alpha\",
         \"defaultBranchRef\": {\"name\": \"main\", \"target\": {\"committedDate\": \"2026-08-30T10:00:00Z\", \"statusCheckRollup\": {\"state\": \"SUCCESS\"}}},
         \"pullRequests\": {\"totalCount\": 1},
         \"refs\": {\"totalCount\": 2, \"nodes\": [
             {\"name\": \"main\", \"target\": {\"committedDate\": \"2026-08-30T10:00:00Z\"}},
             {\"name\": \"feat/login\", \"target\": {\"committedDate\": \"2026-07-01T00:00:00Z\"}}]}}";

    // ── クエリ ────────────────────────────────────────────────────────────

    /// 🔴 設計メモに書いた文字列と**完全一致**する。実際に GitHub が受けた形である。
    #[test]
    fn builds_the_query_from_the_design_note() {
        let expected = "query {\n  \
             r0: repository(owner: \"example-org\", name: \"alpha\") { ...RepoFields }\n  \
             r1: repository(owner: \"example-org\", name: \"beta\") { ...RepoFields }\n\
             }\n\
             fragment RepoFields on Repository { nameWithOwner defaultBranchRef { name target { ... on Commit { committedDate statusCheckRollup { state } } } } pullRequests(states: OPEN) { totalCount } refs(refPrefix: \"refs/heads/\", first: 100) { totalCount nodes { name target { ... on Commit { committedDate } } } } }\n";
        assert_eq!(build_query(&slugs(&["alpha", "beta"])), expected);
    }

    /// bin は空で呼ばないが、**panic しない**（呼び手の前提に頼らない）。
    #[test]
    fn an_empty_query_is_still_valid_text() {
        let query = build_query(&[]);
        assert!(query.starts_with("query {\n}\n"));
        assert!(query.ends_with("}\n"));
    }

    #[test]
    fn the_chunk_size_is_the_measured_one() {
        assert_eq!(REPOS_PER_QUERY, 3_usize);
    }

    // ── 応答（fixture） ───────────────────────────────────────────────────

    fn fixture() -> Vec<Result<RemoteState, RemoteError>> {
        parse_response(&read(RESPONSE), &slugs(&["alpha", "beta", "gamma"]))
    }

    fn today(stale_days: u32) -> Freshness {
        Freshness::new(
            Day::parse_iso8601("2026-09-02").expect("読める"),
            stale_days,
        )
    }

    #[test]
    fn returns_one_result_per_slug_in_order() {
        let found = fixture();
        assert_eq!(found.len(), 3_usize);
        assert!(found.first().expect("在る").is_ok());
        assert!(found.get(1_usize).expect("在る").is_err());
        assert!(found.get(2_usize).expect("在る").is_ok());
    }

    #[test]
    fn reads_the_repository_that_came_back() {
        let found = fixture();
        let first = found.first().expect("在る").as_ref().expect("読める");
        assert_eq!(first.default_branch(), Some("main"));
        assert_eq!(first.ci(), CiState::Success);
        assert_eq!(first.open_pull_requests(), 1_u32);
        // 既定枝 main を除いた feat/login（2026-07-01）だけが古い。
        assert_eq!(
            first.stale_branches(&today(30_u32)),
            StaleCount::Known(1_u32)
        );
    }

    /// `data.r1` が `null` で `errors[].path` が `["r1"]`・`type` が `NOT_FOUND`。
    #[test]
    fn reads_the_repository_that_does_not_exist() {
        let found = fixture();
        assert_eq!(
            found
                .get(1_usize)
                .expect("在る")
                .as_ref()
                .expect_err("読めない"),
            &RemoteError::NotFound
        );
    }

    /// 空のリポジトリ（`defaultBranchRef` が `null`）。
    #[test]
    fn reads_the_empty_repository() {
        let found = fixture();
        let third = found.get(2_usize).expect("在る").as_ref().expect("読める");
        assert_eq!(third.default_branch(), None);
        assert_eq!(third.ci(), CiState::Absent);
        assert_eq!(third.open_pull_requests(), 0_u32);
        assert_eq!(
            third.stale_branches(&today(30_u32)),
            StaleCount::Known(0_u32)
        );
    }

    /// `pullRequests.nodes` のような**頼んでいないフィールドがあっても無視する**。
    #[test]
    fn extra_fields_are_ignored() {
        let source = one(
            "{\"defaultBranchRef\": null, \"pullRequests\": {\"totalCount\": 0, \"nodes\": [{\"number\": 12}]}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        assert_eq!(state(&source).open_pull_requests(), 0_u32);
    }

    // ── 応答（1 リポジトリの失敗） ───────────────────────────────────────

    #[test]
    fn a_rejected_repository_keeps_the_message() {
        let source = "{\"data\": {\"r0\": null}, \"errors\": [{\"type\": \"FORBIDDEN\", \"path\": [\"r0\"], \"message\": \"Resource not accessible\"}]}";
        assert_eq!(
            error(source),
            RemoteError::Rejected(String::from("Resource not accessible"))
        );
    }

    /// `null` なのに `errors` に理由が無い応答は、形として辻褄が合っていない。
    #[test]
    fn a_null_without_a_reason_is_malformed() {
        let source = "{\"data\": {\"r0\": null}, \"errors\": [{\"type\": \"NOT_FOUND\", \"path\": [\"r9\"]}]}";
        assert_eq!(error(source), RemoteError::Malformed(String::from("r0")));
    }

    /// 拒まれたのに文言が無い応答は、伝えるべき言葉が無い。
    #[test]
    fn a_rejection_without_a_message_is_malformed() {
        let source = "{\"data\": {\"r0\": null}, \"errors\": [{\"type\": \"FORBIDDEN\", \"path\": [\"r0\"]}]}";
        assert_eq!(
            error(source),
            RemoteError::Malformed(String::from("errors[].message"))
        );
    }

    /// `defaultBranchRef` が `null` でもオブジェクトでもない応答は読めない。
    #[test]
    fn a_default_branch_ref_of_the_wrong_type_is_malformed() {
        let source = one(
            "{\"defaultBranchRef\": \"main\", \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.defaultBranchRef"))
        );
    }

    #[test]
    fn a_missing_alias_is_malformed() {
        assert_eq!(
            error("{\"data\": {}}"),
            RemoteError::Malformed(String::from("r0"))
        );
    }

    #[test]
    fn an_alias_that_is_not_an_object_is_malformed() {
        assert_eq!(
            error("{\"data\": {\"r0\": \"alpha\"}}"),
            RemoteError::Malformed(String::from("r0"))
        );
    }

    // ── 応答（リクエスト全体の失敗） ─────────────────────────────────────

    /// `{"message":"Bad credentials"}`（実測の形）は全リポジトリの失敗である。
    #[test]
    fn a_request_without_data_rejects_every_repository() {
        let found = results("{\"message\": \"Bad credentials\"}", 3_usize);
        assert_eq!(found.len(), 3_usize);
        for result in found {
            assert_eq!(
                result.expect_err("読めない"),
                RemoteError::Rejected(String::from("Bad credentials"))
            );
        }
    }

    /// `errors` しか無い形でも、文言を拾って伝える。
    #[test]
    fn a_request_error_list_also_carries_the_message() {
        let source = "{\"errors\": [{\"message\": \"timeout\"}]}";
        assert_eq!(
            error(source),
            RemoteError::Rejected(String::from("timeout"))
        );
    }

    /// `data` も文言も無い応答は、何が起きたか言えない。
    #[test]
    fn a_response_without_data_or_message_is_malformed() {
        assert_eq!(error("{}"), RemoteError::Malformed(String::from("data")));
        assert_eq!(
            error("{\"data\": null}"),
            RemoteError::Malformed(String::from("data"))
        );
    }

    // ── 応答（形が違う） ─────────────────────────────────────────────────

    /// 🔴 知らない `state` を黙って飲まない（RS-002）。
    #[test]
    fn an_unknown_ci_state_is_malformed() {
        let source = one(
            "{\"defaultBranchRef\": {\"name\": \"main\", \"target\": {\"statusCheckRollup\": {\"state\": \"NEUTRAL\"}}}, \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from(
                "r0.defaultBranchRef.target.statusCheckRollup.state"
            ))
        );
    }

    /// `statusCheckRollup` が `null` なら「検査が無い」であって、失敗ではない。
    #[test]
    fn a_null_rollup_means_no_checks() {
        let source = one(
            "{\"defaultBranchRef\": {\"name\": \"main\", \"target\": {\"statusCheckRollup\": null}}, \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        let found = state(&source);
        assert_eq!(found.ci(), CiState::Absent);
        assert_eq!(found.default_branch(), Some("main"));
    }

    #[test]
    fn a_count_that_is_not_a_number_is_malformed() {
        let source = one(
            "{\"defaultBranchRef\": null, \"pullRequests\": {\"totalCount\": \"1\"}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.pullRequests.totalCount"))
        );
    }

    /// `u32` に収まらない数も読めないと言う（黙って切り詰めない・RS-007）。
    #[test]
    fn a_count_that_does_not_fit_is_malformed() {
        let source = one(
            "{\"defaultBranchRef\": null, \"pullRequests\": {\"totalCount\": 4294967296}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.pullRequests.totalCount"))
        );
    }

    #[test]
    fn a_broken_committed_date_is_malformed() {
        let source = one(
            "{\"defaultBranchRef\": null, \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 1, \"nodes\": [{\"name\": \"main\", \"target\": {\"committedDate\": \"2026-13-01T00:00:00Z\"}}]}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.refs.nodes[0].target.committedDate"))
        );
    }

    #[test]
    fn a_branch_without_a_name_is_malformed() {
        let source = one(
            "{\"defaultBranchRef\": null, \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 1, \"nodes\": [{\"target\": {\"committedDate\": \"2026-01-01T00:00:00Z\"}}]}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.refs.nodes[0].name"))
        );
    }

    /// 🔴 100 本を超えて切り詰められたら、数を答えない。
    #[test]
    fn more_branches_than_nodes_means_truncated() {
        let source = one(
            "{\"defaultBranchRef\": null, \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 120, \"nodes\": [{\"name\": \"main\", \"target\": {\"committedDate\": \"2020-01-01T00:00:00Z\"}}]}}",
        );
        let freshness = Freshness::new(Day::parse_iso8601("2026-09-02").expect("読める"), 30_u32);
        assert_eq!(
            state(&source).stale_branches(&freshness),
            StaleCount::Truncated
        );
    }

    #[test]
    fn a_missing_default_branch_ref_is_malformed() {
        let source = one(
            "{\"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.defaultBranchRef"))
        );
    }

    #[test]
    fn a_default_branch_ref_without_a_name_is_malformed() {
        let source = one(
            "{\"defaultBranchRef\": {\"target\": {\"statusCheckRollup\": null}}, \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.defaultBranchRef.name"))
        );
    }

    #[test]
    fn a_default_branch_ref_without_a_target_is_malformed() {
        let source = one(
            "{\"defaultBranchRef\": {\"name\": \"main\"}, \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.defaultBranchRef.target"))
        );
    }

    #[test]
    fn a_missing_rollup_is_malformed() {
        let source = one(
            "{\"defaultBranchRef\": {\"name\": \"main\", \"target\": {\"committedDate\": \"2026-01-01T00:00:00Z\"}}, \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 0, \"nodes\": []}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.defaultBranchRef.target.statusCheckRollup"))
        );
    }

    #[test]
    fn missing_refs_are_malformed() {
        let source = one("{\"defaultBranchRef\": null, \"pullRequests\": {\"totalCount\": 0}}");
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.refs"))
        );
    }

    #[test]
    fn refs_without_nodes_are_malformed() {
        let source = one(
            "{\"defaultBranchRef\": null, \"pullRequests\": {\"totalCount\": 0}, \"refs\": {\"totalCount\": 0}}",
        );
        assert_eq!(
            error(&source),
            RemoteError::Malformed(String::from("r0.refs.nodes"))
        );
    }

    /// 全部そろった応答は読める（上の「壊れた」試験の対照）。
    #[test]
    fn the_whole_shape_reads() {
        let found = state(&one(WHOLE));
        assert_eq!(found.default_branch(), Some("main"));
        assert_eq!(found.ci(), CiState::Success);
        assert_eq!(found.open_pull_requests(), 1_u32);
    }

    /// 0 リポジトリで呼んでも panic しない。
    #[test]
    fn no_slugs_means_no_results() {
        assert!(parse_response(&read(RESPONSE), &[]).is_empty());
    }
}
