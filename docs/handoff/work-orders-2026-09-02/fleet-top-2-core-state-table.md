# 作業指示 fleet-top-2 — `fleet-top-core` 後半（porcelain v2・GraphQL・表の整形）

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

## 0. 最初に読む（この順・飛ばさない）

1. `/home/xi/docker/xi-tools/CLAUDE.md` — 第一目的と地雷 6 件
2. `/home/xi/docker/xi-tools/docs/design/fleet-top.md` — **今回の仕様の正本**。「`fleet-top-core` の公開 API」「出力の形」「決定 F-5」
3. `/home/xi/docker/xi-tools/docs/handoff/work-orders-2026-09-02/fleet-top-1-core-parsers.md` — 前回の作業指示（前提になる型: `JsonValue` / `Day` / `GithubSlug`）
4. `/home/xi/docker/xi-tools/fleet-top-core/src/` — **前回作ったコード**。書き方をこれに揃える
5. `/home/xi/docker/xi-tools/docs/coding-rules.md` — 規約。特に RS-001/002/003/004/005/007/008/011/012/013/016/018、QLT-006

## 1. 作るもの

`fleet-top-core` に**後半**を足す。前半の公開 API は変えない（内部の整理は可）。今回もクレートは `no_std`・依存 0 のまま。

### 1-a. `git status --porcelain=v2 --branch` の読み取り

```rust
pub fn parse_porcelain(source: &str) -> Result<LocalState, PorcelainError>;
pub struct LocalState { ... }   // 非公開フィールド
impl LocalState {
    pub fn head(&self) -> &Head;
    pub fn upstream(&self) -> Option<&str>;   // `# branch.upstream` の値。行が無ければ None
    pub fn ahead(&self) -> u32;               // `# branch.ab +A -B` の A。行が無ければ 0
    pub fn behind(&self) -> u32;
    pub fn dirty(&self) -> u32;               // `1` `2` `u` `?` で始まる行の数
}
pub enum Head { Branch(String), Detached }    // `# branch.head (detached)` → Detached。それ以外は Branch(原文)
pub struct PorcelainError { ... }  impl PorcelainError { pub fn kind(&self) -> &PorcelainErrorKind; pub fn line(&self) -> u32 }  // + Display + Error
pub enum PorcelainErrorKind { MissingHead, MalformedHeader, UnexpectedLine }
```

実測した形（`git status --porcelain=v2 --branch`）:

```
# branch.oid (initial)                      ← コミットが無いリポ
# branch.head master
```
```
# branch.oid 5c2528bb47268df1e88c70244a03e2ba0af243cc
# branch.head (detached)
1 A. N... 000000 100644 100644 0000000000000000000000000000000000000000 78981922613b2afb6025042ff6bd878ac1994e85 a
? b
```
```
# branch.oid be1ac856ed7b0fda91270b20c022e7bda6bf8206
# branch.head main
# branch.upstream origin/main
# branch.ab +0 -0
1 .M N... 100644 100644 100644 a7c9904d179471e47f7ef58ee8afbbcd0f3eac72 a7c9904d179471e47f7ef58ee8afbbcd0f3eac72 notes.md
2 R. N... 100644 100644 100644 0cbf1228461a5f32eaaeaae6663ba5a9147d6598 0cbf1228461a5f32eaaeaae6663ba5a9147d6598 R100 new.md	old.md
u UU N... 100644 100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 3333333333333333333333333333333333333333 conflict.txt
? scratch/
! ignored.log
```

- `#` 行は見出し。`# branch.head` が無ければ `MissingHead`。`# branch.ab` の形が `+数 -数` でなければ `MalformedHeader`（行番号つき）。
  知らない `# branch.xxx` / `# stash N` は**無視する**（git が増やす可能性があるため。ただし `#` の後に `branch.` も `stash` も無い行は `UnexpectedLine`）
- `1` `2` `u` `?` は dirty に数える。`!`（ignored）は数えない。それ以外の先頭文字は `UnexpectedLine`
- 空の入力（`--branch` が無い等）は `MissingHead`。末尾の改行の有無は問わない。`\r\n` も受ける
- 枝名は原文のまま（`feat/login` の `/` を含む）

### 1-b. GraphQL のクエリと応答

```rust
pub const REPOS_PER_QUERY: usize = 3;
pub fn build_query(slugs: &[GithubSlug]) -> String;
pub fn parse_response(json: &JsonValue, slugs: &[GithubSlug]) -> Vec<Result<RemoteState, RemoteError>>;

pub struct RemoteState { ... }
impl RemoteState {
    pub fn default_branch(&self) -> Option<&str>;         // defaultBranchRef が null なら None（空のリポ）
    pub fn ci(&self) -> CiState;
    pub fn open_pull_requests(&self) -> u32;
    pub fn stale_branches(&self, freshness: &Freshness) -> StaleCount;
}
pub enum CiState { Success, Failure, Pending, Absent }
pub enum StaleCount { Known(u32), Truncated }
pub enum RemoteError { NotFound, Rejected(String), Malformed(String) }   // + Display + Error
pub struct Freshness { ... }  impl Freshness { pub fn new(today: Day, stale_days: u32) -> Self; pub fn today(&self) -> Day; pub fn stale_days(&self) -> u32 }
```

`build_query` は設計メモ「GraphQL クエリの形」を**文字どおり**返す（テストは完全一致）。エイリアスは `r0` から順、`slugs` の並びと同じ。
`slugs` が空なら `query {\n}\nfragment ...` を返す（bin は空で呼ばないが、panic しない）。改行は `\n`。末尾改行あり。

`parse_response` は `slugs.len()` 個の結果を**同じ順で**返す:

| 応答 | 結果 |
| --- | --- |
| `data.rN` がオブジェクト | `Ok(RemoteState)`。必要なフィールドが無い・型が違う → `Err(Malformed("<field path>"))`（例: `Malformed("r1.pullRequests.totalCount")`） |
| `data.rN` が `null` で `errors[]` に `path == ["rN"]` の要素がある | `type` が `"NOT_FOUND"` → `NotFound`。それ以外 → `Rejected(message)` |
| `data.rN` が `null` で対応する `errors` が無い、または `data.rN` が無い | `Malformed("rN")` |
| `data` が無い | 全要素 `Rejected(message)`。`message` も無ければ `Malformed("data")` |

`RemoteState` の中身:

- `defaultBranchRef`: `null` → `default_branch() = None`・`ci() = Absent`。オブジェクトなら `name` と `target.statusCheckRollup`
- `statusCheckRollup`: `null` → `Absent`。`state` が `SUCCESS` → `Success`、`FAILURE` / `ERROR` → `Failure`、`PENDING` / `EXPECTED` → `Pending`。
  **それ以外の文字列は `Malformed("rN.defaultBranchRef.target.statusCheckRollup.state")`**（閉じた集合の外を黙って飲まない・RS-002）
- `pullRequests.totalCount`: `as_u64` → `u32`（収まらなければ `Malformed`）
- `refs.nodes[]`: `name` と `target.committedDate`（`Day::parse_iso8601`。失敗は `Malformed`）。`refs.totalCount > nodes.len()` なら **切り詰められている**ことを記録し、`stale_branches` は `Truncated` を返す
- `stale_branches(freshness)`: 既定枝**以外**の枝のうち `today.days_since(committed) > stale_days` のものの数（`days_since` が None＝未来の日付は数えない）。既定枝が None なら全枝が対象

### 1-c. 表

```rust
pub enum LocalReport { State(LocalState), Unavailable }
pub enum RemoteReport { State(RemoteState), NotOnGithub, Unavailable }
pub struct Row { ... }
impl Row {
    pub fn new(name: String, local: LocalReport, remote: RemoteReport) -> Self;
    pub fn name(&self) -> &str;
    pub fn is_complete(&self, freshness: &Freshness) -> bool;   // `?` を 1 つも出さない行なら true（Truncated も `?` なので false）
}
pub fn render(rows: &[Row], freshness: &Freshness) -> String;
```

`render` の規則（設計メモ「出力の形」。**完全一致で試験する**）:

- 行は `name()` の**バイト順**に並べ替える（入力順に依存しない）。同名は入力順
- 見出し `REPO  BRANCH  DIRTY  AHEAD/BEHIND  PR  CI  STALE`。列は 2 空白区切り・左寄せ・列幅はその列の最大**文字数**（`chars().count()`・見出し含む）。
  最終列 `STALE` は詰めない。**行末に空白を出さない**。各行の末尾は `\n`。行が 0 なら見出しだけ
- 各セル:

| 列 | `LocalReport::State` / `RemoteReport::State` | `Unavailable` | `NotOnGithub` |
| --- | --- | --- | --- |
| BRANCH | `Head::Branch(b)` → `b`、`Detached` → `(detached)` | `?` | — |
| DIRTY | 0 → `-`、それ以外は数 | `?` | — |
| AHEAD/BEHIND | upstream 無し → `(none)`。両方 0 → `-`。`+A`（behind 0）/ `-B`（ahead 0）/ `+A/-B` | `?` | — |
| PR | 0 → `-`、それ以外は数 | `?` | `n/a` |
| CI | `Success` → `ok`、`Failure` → `FAIL`、`Pending` → `...`、`Absent` → `-` | `?` | `n/a` |
| STALE | `Known(0)` → `-`、`Known(n)` → `n`、`Truncated` → `?` | `?` | `n/a` |

設計メモの例（4 行）をそのまま fixture にして完全一致を見る。`beta` は `+2/-1`・dirty 3・PR 1・FAIL・stale 2、`gamma` は detached・upstream 無し・GitHub に無い、`delta` は local も remote も `Unavailable`。

## 2. テスト（QLT-007。これが無いと受け取らない）

1. **porcelain**: 上の実測 3 形（initial / detached＋変更 / upstream あり＋全種類の行）を fixture 文字列にして `head` / `upstream` / `ahead` / `behind` / `dirty` を検証。
   `\r\n`・末尾改行なし・空入力（`MissingHead`）・`# branch.ab +x -1`（`MalformedHeader` と行番号）・知らない `#` 見出しの無視・`# stash 2` の無視・`z ...`（`UnexpectedLine`）
2. **build_query**: 2 リポで設計メモの文字列と**完全一致**。0 リポで panic しないこと
3. **parse_response**: `testdata/graphql-response.json`（前回の fixture。`pullRequests.nodes` を持っていても無視されること）で `r0` が `Ok`（`main` / `Success` / PR 1 / 枝 2 本）、
   `r1` が `NotFound`、`r2` が `Ok` で `default_branch() = None`・`Absent`。加えて手書きの JSON で: `Rejected`（`type` が `FORBIDDEN` 等）・
   `data` 無し＋`message`（全要素 `Rejected`）・`data` も `message` も無し・`state` が知らない文字列・`totalCount` が文字列・`committedDate` が壊れている・
   `totalCount > nodes.len()` で `Truncated`
4. **stale_branches**: `today` と `stale_days` を動かして境界（ちょうど `stale_days` 日は数えない・`+1` 日は数える）・既定枝を除くこと・未来日を数えないこと・既定枝 None で全枝が対象
5. **render**: 設計メモの 4 行の例と**完全一致**。行 0（見出しだけ）。並び替え（逆順で渡して名前順で出ること）。列幅が日本語（全角）の枝名でも文字数で揃うこと。
   `+A` だけ・`-B` だけ・`(none)`・`...`・`Truncated` → `?` の各セル。`is_complete` が `Unavailable` / `Truncated` で false、それ以外で true

## 3. 完了条件

- `make check` が緑。**最後に必ず `make check` を通す**
- `cargo test -p fleet-top-core` のテスト件数（前回からの増分も）を報告する
- **コミットはしない**。`git status --short` で変更一覧を報告する
- 報告に含める: 変更ファイル一覧・テスト件数・`make check` の末尾出力・`#[expect]` の全列挙・規約で詰まった箇所とどう解いたか・仕様で曖昧だった点と自分の解釈・満たせなかった点

## 4. 🔴 やってはいけないこと

- `Cargo.toml`（root）の `[workspace.lints]`・`clippy.toml`・`Makefile`・`deny.toml`・`docs/coding-rules.md`・`xtask/` を**変更しない**。通らないなら設計を変える。それでも通らないなら理由を書いて止めて報告する
- 前半の公開 API（`parse_json` / `JsonValue` / `Day` / `GithubSlug` …）の**シグネチャを変えない**（必要なら報告して止める）
- `scopegrep` / `scopegrep-core` を触らない。`README*` / `CHANGELOG.md` / `docs/` を触らない
- fixture は架空データ（地雷 5）。**実在のリポ名・枝名・PR タイトルを使わない**
- `fleet-top`（bin）はまだ作らない（次の作業指示）
- 時間がかかっても仕様を勝手に狭めない。狭めるなら報告に明記する

## 5. ヒント

- 型ごとに 1 ファイル（CNF-003）。`Row` / `LocalReport` / `RemoteReport` / `Freshness` / `StaleCount` / `CiState` / `RemoteError` / `RemoteState` / `LocalState` / `Head` / `PorcelainError` / `PorcelainErrorKind` はそれぞれ別ファイル
- `render` はセルの文字列を先に全部作ってから列幅を取る（2 パス）。1 関数 60 行を超えるなら「セル化」「列幅」「行の連結」に割る
- `String` の右詰めは `format!("{:<width$}")` が `alloc::format!` で使える。ただし `width` は**文字数**なので、`{:<w$}` は文字数で詰める（Rust の `Display` の幅は `char` 数）。全角は 1 文字と数える（端末上の見た目幅は v1 では扱わない）
- `RS-002`: `match` に `_` を書かない。`CiState` の変換は文字列 → enum の `match` で全文字列を列挙し、それ以外は `Malformed`（文字列の `match` の `_` は enum ではないので `wildcard_enum_match_arm` の対象外。`other =>` と名前を付けて束縛すると読みやすい）
- `u32::try_from(u64)` の失敗は `Malformed` に写す（`map_err` で `Malformed(...)` を作る。`map_err_ignore` が deny なので `|_|` で捨てない——`|error|` と受けて使う、または `ok().ok_or_else(...)`）
