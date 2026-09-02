# fleet-top 設計メモ

**状態: 設計確定（2026-09-02・試作の実測に基づく）。実装は作業指示 `docs/handoff/work-orders-2026-09-02/fleet-top-*.md` で進める。**
新しいツールを足す判断そのものは [ADR 0003](../adr/0003-fleet-top-fetches-github-via-chunked-graphql.md)。

## 解く問題

数十の git リポジトリを並べて作業していると、「今どの枝にいて、未コミットがあり、
リモートとどれだけずれ、open な PR と CI の状態がどうか」を**全リポについて**知りたい場面が
1 日に何度もある。それは毎回その場で書き捨てのループになり、**遅い**。

実測（2026-09-01・統合リナ）:

| | 数 |
| --- | --- |
| git リポ | 59 |
| ローカルの状態を直列で（`git status` 等） | 1.5 秒 |
| `gh api` 1 本 | 0.67 秒 |
| `gh api` を直列で 5 本 | 3.4 秒 |
| フリート 42 リポ × 3 種（repo 設定 / open PR / CI 状態）＝ 126 本 | **≒ 84 秒**（外挿） |

🔴 **84 秒かかるコマンドは打たれない。** 打たれないので、見えるはずのもの（期限切れ・4 日間 0 艦の監査）が見えない。
道具の目的は「速くて気持ちいい」ではなく、**打たれるコマンドと打たれないコマンドの境界を越えること**。

## 実測（2026-09-02・試作。スクラッチパッドの使い捨て Rust で計測、リポには入れていない）

環境: WSL2・20 論理コア・`gh` 認証済み・ローカル 60 リポ（`/home/xi/docker` 直下）・GitHub 側 60 リポ（archived を除く）。
試作は `std::thread::scope` ＋ `Mutex<VecDeque>` のワーカープールで `gh` / `git` をサブプロセスとして叩いた。

### GitHub 側 — REST（`gh api` 1 本 = 1 エンドポイント。リポ × 3 本）

| 形 | 本数 | 壁時計 |
| --- | --- | --- |
| 直列 | 21（7 リポ） | **15.46 s**（0.74 s/本。126 本に外挿すると **93 s**） |
| 8 並列 | 126（42 リポ） | 10.72 s |
| 16 並列 | 126 | 6.31 s |
| 32 並列 | 126 | 3.53 s |
| 48 並列 | 126 | 2.84 s |
| 64 並列 | 126 | 2.38 s |

並列度にほぼ線形で縮む（レイテンシ律速）。秒間 50 本近く投げても secondary rate limit には当たらなかったが、
**rate limit を 126 点消費する**（core 5,000 点/時）。1 回で 2.5%。
`gh` 自体の起動が 1 回 0.28 s（`gh --version` × 5 = 1.42 s）、`curl` 単体 0.42 s に対し `gh api` 0.67 s。

### GitHub 側 — GraphQL（1 リクエストに複数リポをエイリアスで並べる）

| 形 | 壁時計 |
| --- | --- |
| **1 本**に 42 リポ（既定枝の CI・open PR・枝一覧つき） | **8.87 s** |
| 1 本に 60 リポ | **HTTP 502**（サーバ側タイムアウト） |
| 1 本に 42 リポ、枝一覧を外す | 5.74 s / 5.87 s |
| 42 リポを **7 リポ × 6 本**で並列 | 1.66 / 2.40 / 1.76 / 2.00 s |
| 42 リポを **3 リポ × 14 本** | 1.60 s |
| 42 リポを 14 リポ × 3 本 | 2.25 s |
| 42 リポを 21 リポ × 2 本 | 3.15 s |
| 60 リポを **3 リポ × 20 本** | **1.35 / 1.49 / 1.45 s** |
| 60 リポを 6 リポ × 10 本 | 2.69 / 1.92 / 1.76 / 1.72 / 1.96 s |
| 60 リポを 10 リポ × 6 本 | 2.02 s |

🔑 **GraphQL は 1 本にまとめるほど遅い**（サーバ側で直列に解決している挙動）。**小さく割って並列に投げる**のが速い。
GraphQL の rate limit は REST と**別枠**（5,000 点/時）で、1 リクエスト = 1 点。60 リポで 20 点。
枝一覧（`refs` 100 本＋各コミット日）を含めても分割並列なら差が出ない。

⚠️ `gh api graphql` は **`errors` が 1 件でもあると終了コード 1 を返すが、stdout の `data` には成功分が入っている**
（実測: 存在しないリポを 1 つ混ぜると `data.b: null` ＋ `errors[0].path: ["b"]`・`type: NOT_FOUND`）。
終了コードで捨てると、1 リポの失敗で同じリクエストの他のリポも消える。**stdout を読んで `errors[].path` で潰す。**

### ローカル側（`git status --porcelain=v2 --branch` / `worktree list` / `for-each-ref`。60 リポ × 3 本 = 180）

| 形 | 壁時計 |
| --- | --- |
| 直列 | 1.26 s（初回）/ 0.30 s（キャッシュ後） |
| 8 / 16 / 32 並列 | 0.05 / 0.03〜0.05 / 0.05〜0.08 s |

ローカルは並列にすれば誤差。全体の壁時計は GitHub 側で決まる。

### 結論

**3 リポ × N 本の GraphQL を並列に投げる形で、60 リポが 1.4〜1.5 秒。** 境界（3 秒）を越える。
REST 64 並列（2.4 s・126 点）より速く、rate limit の消費は 1/6。

## 決定（F-1〜F-5）

| | 論点 | 決定 | 却下した案と理由 |
| --- | --- | --- | --- |
| F-1 | 表示 | **1 回出力して終わる CLI**。表を stdout、要約 1 行（リポ数・所要秒）を stderr | TUI（`ratatui`）: 1.5 秒で返るものを常駐させる理由が無い。数十 crate 入る。`--watch` も v1 では作らない（`watch fleet-top` で足りる） |
| F-2 | GitHub の取り方 | **`gh api graphql` をサブプロセスで叩く**。認証を借りる。3 リポずつ 1 リクエスト | `octocrab` 等: token の置き場と依存（`tokio`・`reqwest`・TLS）を抱える。`curl` 直叩き: 0.25 s 速いが token 管理が要る。REST: 上表のとおり遅く rate limit を 6 倍食う |
| F-3 | 並行実行 | **`std::thread::scope` ＋ `Mutex<VecDeque>` のワーカープール（上限 32）**。依存 0 | `tokio`: サブプロセスの待ち合わせに非同期ランタイムは要らない。試作が `std` だけで 1.4 秒を出した |
| F-4 | 対象の決め方 | **引数のディレクトリ直下**で `.git` を持つものだけ（既定 `.`）。再帰しない。GitHub の owner/name は `git remote get-url origin` から読む | 設定ファイル: 置き場と書式が要る。`--depth`: フリートの置き方（直下に並ぶ）で困っていない。困ってから |
| F-5 | 出力の決定性 | **表示順はディレクトリ名のバイト順**。取れなかった値は `?`、GitHub に無いリポは `n/a`、ゼロ・該当なしは `-`。**黙って空にしない。終了コードで `?` の有無を返す** | 失敗行を消す: この道具が生まれた事故（片方だけ見て判断）と同じ形 |

🔴 **依存は 0 のまま。** F-2・F-3 で依存を足さない選択をしたので、F-3 に予告した「依存の ADR」は不要になった。
ただし新しいツールを足す判断は ADR を要する（`docs/adr/README.md`）ので、[ADR 0003](../adr/0003-fleet-top-fetches-github-via-chunked-graphql.md) がそれである。
JSON の読み取り（GraphQL の応答）は **`fleet-top-core` に手書きの JSON パーサ**を置く（RFC 8259 全体。小さい）。
`serde_json` を入れると 5 crate 入り、`no_std` の中核が `alloc` だけで閉じなくなる。

## アーキテクチャ

```
fleet-top-core（#![no_std] + alloc・依存 0）      fleet-top（bin・依存 0）
─────────────────────────────────────────         ─────────────────────────────────
JSON パーサ（RFC 8259）                            引数の解釈（配線点・RS-015）
git porcelain v2 の読み取り → LocalState           ディレクトリ直下の走査
remote URL → GithubSlug                           git / gh のサブプロセス起動（並列）
GraphQL クエリの組み立て（3 リポ/本）              「今日」の取得（SystemTime）
GraphQL 応答の読み取り → RemoteState               core に渡して表を受け取り、出力
ISO 8601 → 日数、鮮度の判定                        終了コード
表の整形（列幅・記号・並び順）
```

**ARC-003 について。** `scopegrep` と違い、この道具は I/O が本体である。それでも中核を `no_std` にできるのは、
**I/O の結果を文字列として受け取り、状態の解釈と表の整形だけを core に置く**からである。
サブプロセスの起動・並列・時刻の取得は bin に残る。core は「同じ入力（文字列）から同じ表」を返す純粋関数の集まりで、
**fixture の文字列だけで全部テストできる**。これが core を切る理由であり、`no_std` にできない部分（起動・待ち合わせ・時刻）は
bin の配線点に閉じる。

## `fleet-top-core` の公開 API

```rust
// JSON（GraphQL 応答の読み取り用。汎用）
pub fn parse_json(source: &str) -> Result<JsonValue, JsonError>;
pub enum JsonValue { Null, Bool(bool), Number(JsonNumber), String(String), Array(Vec<JsonValue>), Object(Vec<(String, JsonValue)>) }
//   ⚠️ Object は挿入順の Vec（RS-016。BTreeMap にすると応答の順が消える。重複キーは後勝ち）
impl JsonValue { pub fn get(&self, key: &str) -> Option<&JsonValue>; pub fn as_str(&self) -> Option<&str>; pub fn as_array(&self) -> Option<&[JsonValue]>; ... }

// ローカル
pub fn parse_porcelain(source: &str) -> Result<LocalState, PorcelainError>;   // `git status --porcelain=v2 --branch` の出力
impl LocalState { head() -> &Head; upstream() -> Option<&str>; ahead() -> u32; behind() -> u32; dirty() -> u32 }
pub enum Head { Branch(String), Detached }
pub fn parse_remote_url(url: &str) -> Option<GithubSlug>;   // https://github.com/o/n(.git) / git@github.com:o/n(.git) / ssh://git@github.com/o/n
impl GithubSlug { owner() -> &str; name() -> &str }

// GitHub
pub const REPOS_PER_QUERY: usize = 3;                        // 実測で決めた値。上表
pub fn build_query(slugs: &[GithubSlug]) -> String;          // エイリアス r0..rN ＋ fragment RepoFields（実測で動作確認済み）
pub fn parse_response(json: &JsonValue, slugs: &[GithubSlug]) -> Vec<Result<RemoteState, RemoteError>>;
impl RemoteState { default_branch() -> Option<&str>; ci() -> CiState; open_pull_requests() -> u32; stale_branches(&self, freshness: &Freshness) -> StaleCount }
pub enum CiState { Success, Failure, Pending, Absent }       // SUCCESS / FAILURE・ERROR / PENDING・EXPECTED / null
pub enum StaleCount { Known(u32), Truncated }                // refs.totalCount > nodes の数なら Truncated（100 本超は数えない）
pub enum RemoteError { NotFound, Rejected(String), Malformed(String) }   // NOT_FOUND / GitHub の message 原文 / 応答の形が想定外
pub struct Freshness { ... }  impl Freshness { pub fn new(today: Day, stale_days: u32) -> Self }

// 日付
pub struct Day(...);  impl Day { pub fn from_unix_seconds(secs: u64) -> Self; pub fn parse_iso8601(s: &str) -> Option<Self>; pub fn days_since(self, earlier: Self) -> Option<u32> }

// 表
pub enum LocalReport { State(LocalState), Unavailable }
pub enum RemoteReport { State(RemoteState), NotOnGithub, Unavailable }
pub struct Row { ... }  impl Row { pub fn new(name: String, local: LocalReport, remote: RemoteReport) -> Self; pub fn is_complete(&self, freshness: &Freshness) -> bool }
pub fn render(rows: &[Row], freshness: &Freshness) -> String;   // 名前のバイト順に並べ替えてから整形。末尾改行あり
```

型ごとに 1 ファイル（CNF-003）。`HashMap` 禁止。`Default` 禁止。数値リテラルは型付き。

GraphQL クエリの形（`build_query` が返す。fragment で 1 回だけフィールドを書く）:

```graphql
query {
  r0: repository(owner: "example-org", name: "alpha") { ...RepoFields }
  r1: repository(owner: "example-org", name: "beta") { ...RepoFields }
}
fragment RepoFields on Repository { nameWithOwner defaultBranchRef { name target { ... on Commit { committedDate statusCheckRollup { state } } } } pullRequests(states: OPEN) { totalCount } refs(refPrefix: "refs/heads/", first: 100) { totalCount nodes { name target { ... on Commit { committedDate } } } } }
```

応答の形は 3 通りある（すべて実測）:

| 形 | 意味 | 扱い |
| --- | --- | --- |
| `data.rN` がオブジェクト | 取れた | `RemoteState` |
| `data.rN` が `null` ＋ `errors[].path == ["rN"]` | そのリポだけ失敗（`type: NOT_FOUND` 等） | `NotFound` / `Rejected(message)` |
| `data` が無く `message` だけ（`{"message":"Bad credentials"}`） | リクエスト全体が失敗 | 全リポ `Rejected(message)` |

## 出力の形（完全一致で試験する）

```
REPO   BRANCH      DIRTY  AHEAD/BEHIND  PR   CI    STALE
alpha  main        -      -             -    ok    -
beta   feat/login  3      +2/-1         1    FAIL  2
delta  ?           ?      ?             ?    ?     ?
gamma  (detached)  -      (none)        n/a  n/a   n/a
```

（`delta` はローカルも GitHub も読めなかった行。ローカルが読めなければ枝名も無いので、**行全体が `?`** になる。
初版の例は `delta main ? …` と `gamma` の後に `delta` を置いていて、型ともバイト順とも矛盾していた——実装リナの指摘で 2026-09-02 に訂正）

- 列は 2 空白区切り・左寄せ。列幅はその列の最大長（見出し含む・文字数）。最終列は詰めない。行末の空白は出さない
- `REPO`: ディレクトリ名。バイト順
- `BRANCH`: `# branch.head`。detached は `(detached)`
- `DIRTY`: 変更・未追跡・衝突の**エントリ数**（porcelain v2 の `1` `2` `u` `?` 行の合計）。0 は `-`
- `AHEAD/BEHIND`: `# branch.ab +A -B` から。両方 0 は `-`。片方だけなら `+2` / `-1`、両方なら `+2/-1`。upstream 無し（`# branch.upstream` 行が無い）は `(none)`
- `PR`: open PR 数。0 は `-`
- `CI`: 既定枝の先頭コミットの `statusCheckRollup.state`。`ok` / `FAIL` / `...`（pending）/ `-`（無し）
- `STALE`: 既定枝以外のリモート枝のうち、最終コミットが `--stale-days`（既定 30）より古いものの数。0 は `-`
- `n/a`: origin が GitHub でない（または origin が無い）。GitHub の 3 列に出す
- `?`: 取れなかった（`gh` 不在・失敗・応答が読めない・`git` 失敗）。理由は stderr に 1 行ずつ（`fleet-top: <repo>: <理由>`）

stderr の要約（最後の 1 行）: `fleet-top: 60 repos, 45 on GitHub, 1.4s`

## CLI

```
fleet-top [DIR] [--stale-days N] [--no-github]
fleet-top --help / --version
```

| 終了コード | 意味 |
| --- | --- |
| 0 | 全行が確定した（`?` が無い） |
| 1 | `?` を含む行がある（表は出ている） |
| 2 | 使い方の誤り・`DIR` が読めない |

`--no-github`: GitHub の 3 列を `n/a` にして `gh` を起動しない（オフライン用）。

## 規約から来る制約

| 規則 | 影響 |
| --- | --- |
| ARC-001 | 1 ツール = 1 クレート。`scopegrep-core` とコードを共有しない（JSON パーサも fleet-top-core に置く） |
| ARC-003 | 上記「アーキテクチャ」。core は文字列を受けて表を返す。時刻は `Day` の値として受け取る |
| RS-016 | 反復順は決定的。並列で返ってきた結果は名前順に並べ直す。JSON の Object は挿入順の `Vec` |
| RS-014 / RS-015 | 出力は `output.rs` だけ。環境（引数・cwd・時刻・サブプロセス）は bin だけ |
| QLT-009 | 上の実測表が「速い」の根拠。README にはこの数字だけを書く。`docs/benchmarks/fleet-top.md` に写す |

## README の例について

`scopegrep` の `tests/readme.rs` は README の例を実行して照合するが、`fleet-top` の出力は**その時点の GitHub とローカルの状態**で、
再現しない。README の例は**実行日を添えた実出力**とし、表の整形は `fleet-top-core` の `render` のテストが fixture で完全一致を見る。
「動く例」の保証は core のテストに移る。この違いは README に書く。

## 非目標

- GitHub 以外（GitLab 等）
- リポの操作（fetch・checkout・merge）。**見るだけ**。`git fetch` もしない（ahead/behind は手元の追跡枝との差）
- フリート固有の概念（kit・weekly-audit・艦の名前）を道具に持ち込まない。汎用の「リポの状態」だけ
- TUI・`--watch`・`--json`（v1）。困ってから
- worktree の一覧（試作で測ったが v1 の列に入れない。直下に無い worktree は対象外）

## 却下した案

| 案 | 理由 |
| --- | --- |
| REST を高並列で叩く | 64 並列で 2.38 s・rate limit 126 点。GraphQL 分割並列が 1.4 s・20 点 |
| GraphQL 1 本に全リポ | 42 リポで 8.87 s、60 リポで 502。まとめるほど遅い |
| `serde_json` | 5 crate。core の `no_std`・依存 0 が崩れる。JSON は手で書ける大きさ |
| `tokio` | サブプロセス待ちに非同期ランタイムは要らない。`std::thread` で 1.4 s |
| `ratatui` TUI | 1.5 秒で返るものを常駐させる理由が無い |
| `gh api --jq` で TSV に潰す | 構造の解釈が jq 文字列の中に隠れる。タイトルの改行・タブで壊れる |
| README の例を `tests/readme.rs` で照合 | 出力が時刻とネットワークに依存する。core の fixture テストで代替 |
