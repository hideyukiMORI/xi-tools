# 作業指示 fleet-top-1 — `fleet-top-core` 前半（JSON パーサ・`Day`・remote URL）

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

## 0. 最初に読む（この順・飛ばさない）

1. `/home/xi/docker/xi-tools/CLAUDE.md` — 第一目的と地雷 6 件
2. `/home/xi/docker/xi-tools/docs/design/fleet-top.md` — **今回の仕様の正本**。特に「アーキテクチャ」「`fleet-top-core` の公開 API」
3. `/home/xi/docker/xi-tools/docs/adr/0003-fleet-top-fetches-github-via-chunked-graphql.md` — なぜ依存 0 で JSON を手書きするか
4. `/home/xi/docker/xi-tools/docs/coding-rules.md` — 規約。特に RS-001/002/003/005/007/008/011/012/013/016/018、ARC-003、QLT-006
5. `/home/xi/docker/xi-tools/scopegrep-core/src/` — **この規約下で書かれた `no_std` クレートの実例**。`lib.rs`（crate doc と `mod` 宣言だけ）・`line_number.rs`（newtype の形）・`scanner.rs`（状態を小さな型に分けて関数を短く保つ形）・`parse_error.rs` / `parse_error_kind.rs`（エラー型の形）に倣う
6. `/home/xi/docker/xi-tools/scopegrep-core/Cargo.toml` / root `Cargo.toml` / `clippy.toml` / `Makefile`

## 1. 作るもの

workspace に **`fleet-top-core`** クレート（lib）を足す。`/home/xi/docker/xi-tools/fleet-top-core/`。今回は**前半**（純粋なパーサ 3 つ）だけ。
porcelain v2・GraphQL・表の整形は次の作業指示で足すので、**今回はモジュールを作らない**。

- `#![no_std]` + `extern crate alloc`。**依存 0**（`[dependencies]` は空）
- `Cargo.toml` は `scopegrep-core/Cargo.toml` に倣う。`name = "fleet-top-core"`・`version = "0.0.0"`・
  `description = "no_std core of fleet-top: parses git/GitHub output into repository state and renders the table"`・
  `readme = "../README.md"`（理由は `scopegrep-core/Cargo.toml` のコメントのとおり）・`keywords` / `categories` は妥当なものを・`[lints] workspace = true`
- root `Cargo.toml` の `members` に足す（`"fleet-top-core"` を `"scopegrep-core"` の後ろ・`"xtask"` の前）。`Cargo.lock` は `cargo` に更新させてコミット対象にする
- `lib.rs` は crate doc と `mod` 宣言だけ。crate doc に設計メモへの案内と「この crate は文字列を受けて値を返すだけで、I/O・時刻・環境に触らない」ことを書く
- **型ごとに 1 ファイル**（CNF-003）。`mod.rs` を作らない。`pub use` を書かない。`Default` を実装・derive しない。`HashMap` を使わない

### 1-a. JSON パーサ（RFC 8259 全体）

```rust
pub fn parse_json(source: &str) -> Result<JsonValue, JsonError>;

pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),   // 🔴 挿入順の Vec。BTreeMap にしない（応答の順を保つ・RS-016）
}
impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue>;   // Object 以外は None。重複キーは**最後の**ものを返す
    pub fn as_str(&self) -> Option<&str>;
    pub fn as_bool(&self) -> Option<bool>;
    pub fn as_array(&self) -> Option<&[JsonValue]>;
    pub fn as_object(&self) -> Option<&[(String, JsonValue)]>;
    pub fn as_number(&self) -> Option<&JsonNumber>;
    pub fn is_null(&self) -> bool;
}

pub struct JsonNumber(...);   // 字句を**原文のまま**保持する（f64 に落とさない。整数の精度を失わない）
impl JsonNumber {
    pub fn as_u64(&self) -> Option<u64>;   // 小数点・指数・負号が無く、u64 に収まるときだけ Some
    pub fn as_i64(&self) -> Option<i64>;
    pub fn as_f64(&self) -> f64;           // `core::str::FromStr for f64`。字句は文法で検証済みなので失敗しない（失敗経路は Result に上げず、`Option` を返す形にしてもよい。**`unwrap` は書けない**）
    pub fn lexeme(&self) -> &str;
}

pub struct JsonError { ... }
impl JsonError { pub fn offset(&self) -> usize; pub fn kind(&self) -> &JsonErrorKind }   // offset は**文字数**（`char` の数。バイトではない）
// + Display + core::error::Error
pub enum JsonErrorKind {
    UnexpectedEnd,
    UnexpectedCharacter(char),
    InvalidEscape,
    InvalidUnicodeEscape,     // \u の後が 16 進 4 桁でない・孤立サロゲート
    ControlCharacterInString, // U+0000〜U+001F がエスケープ無しで文字列に現れた
    InvalidNumber,            // 先頭 0（`01`）・`.5`・`1.`・`1e`・`-`
    TrailingCharacters,       // 値の後に空白以外がある
    TooDeep,                  // 入れ子が上限を超えた
}
```

振る舞い:

- 空白は ` ` `\t` `\n` `\r` のみ（RFC 8259 §2）。先頭・末尾・トークン間で許す
- 文字列: エスケープは `\"` `\\` `\/` `\b` `\f` `\n` `\r` `\t` `\uXXXX`。**サロゲートペア（`😀` → 😀）を結合する**。孤立サロゲートは `InvalidUnicodeEscape`。
  エスケープ無しの制御文字（U+0000〜U+001F）は `ControlCharacterInString`。それ以外の非 ASCII はそのまま通す
- 数: RFC の文法（`-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`）を**厳密に**。外れたら `InvalidNumber`
- 入れ子の上限は **128**（配列とオブジェクトの深さ）。超えたら `TooDeep`。**再帰で書くなら深さを引数で持ち回って上限で止める**（スタックを溢れさせない）
- 上位の値 1 つの後は空白だけ。残れば `TrailingCharacters`
- 重複キー: 両方とも `Vec` に残す（順序を保つ）。`get` は最後のものを返す
- **panic しない**。どの入力でも `Ok` か `Err` を返す（forbid 群が守るが、添字を `get` に置き換えるだけでなく、**論理としても** 落ちる経路を作らない）

### 1-b. `Day`（1970-01-01 からの日数）

```rust
pub struct Day(...);   // 非公開フィールド
impl Day {
    pub fn from_unix_seconds(seconds: u64) -> Self;         // 86_400 で割る
    pub fn parse_iso8601(text: &str) -> Option<Self>;       // `YYYY-MM-DD` または `YYYY-MM-DDThh:mm:ssZ`。それ以外は None
    pub fn days_since(self, earlier: Self) -> Option<u32>;  // self < earlier なら None
    pub fn get(self) -> u32;
}
// + Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord
```

- 暦の変換は **proleptic Gregorian の civil-from-days / days-from-civil**（Howard Hinnant の整数アルゴリズム）。年は 1970〜9999 の範囲で受ける（範囲外は None）
- 月・日の範囲を検証する（`2026-02-30` は None。うるう年 `2024-02-29` は Some、`2100-02-29` は None）
- 時刻部は `T` `hh:mm:ss` `Z` の形だけ検証し、値は日に影響しない（`Z` 以外のタイムゾーンは None）。GitHub GraphQL の `committedDate` / `updatedAt` はこの形で返る（実測: `2025-11-09T16:25:19Z`）
- 🔴 `as` を書かない。`u32` / `i64` の間は `From` / `TryFrom`

### 1-c. remote URL → `GithubSlug`

```rust
pub fn parse_remote_url(url: &str) -> Option<GithubSlug>;
pub struct GithubSlug { ... }   // 非公開フィールド
impl GithubSlug { pub fn new(owner: &str, name: &str) -> Option<Self>; pub fn owner(&self) -> &str; pub fn name(&self) -> &str }
// + Debug, Clone, PartialEq, Eq
```

受ける形（末尾の改行・空白は trim。末尾の `/` は 1 つまで許す。`.git` は 1 回だけ剥がす）:

| 入力 | 結果 |
| --- | --- |
| `https://github.com/alpha/beta` | `alpha` / `beta` |
| `https://github.com/alpha/beta.git` | `alpha` / `beta` |
| `git@github.com:alpha/beta.git` | `alpha` / `beta` |
| `ssh://git@github.com/alpha/beta` | `alpha` / `beta` |
| `https://gitlab.com/alpha/beta` | None |
| `https://github.com/alpha` | None（name が無い） |
| `https://github.com/alpha/beta/extra` | None |
| `https://github.com/al pha/beta` | None（`[A-Za-z0-9_.-]` 以外を含む） |
| `` （空） | None |

`GithubSlug::new` は owner / name が空または `[A-Za-z0-9_.-]` 以外を含むとき None。`.` だけ・`..` も None。

## 2. テスト（QLT-007。これが無いと受け取らない）

`#[cfg(test)]` モジュール内では `extern crate std;` を使ってよい（`clippy.toml` がテストの `unwrap` / 添字を免除している）。

必須:

1. **JSON**: 値 6 種・入れ子・空の配列とオブジェクト・全エスケープ・サロゲートペアの結合・孤立サロゲートの拒否・制御文字の拒否・
   数の文法（`-0` `1e5` `1.5E-3` を受け、`01` `.5` `1.` `1e` `-` を拒む）・`as_u64` の境界（`18446744073709551615` は Some、`18446744073709551616` は None、`1.0` は None、`-1` は None）・
   空白の位置・先頭と末尾の余分な文字・`TooDeep`（129 段の `[`）が **panic せず** Err になること・重複キーで `get` が最後を返すこと・
   `offset()` が文字数であること（日本語を含む入力で確認）・`Display` のメッセージにオフセットと種別が入ること
2. **fixture**: `fleet-top-core/testdata/graphql-response.json` を**架空のリポ名で手書き**し、`include_str!` で読んで `parse_json` が `Ok` を返し、
   `data.r0.nameWithOwner` 等に `get` で辿り着けること。形は次のとおり（GitHub GraphQL の実応答と同じ構造。**実データからコピーしない**・地雷 5）:
   ```json
   {"data":{"r0":{"nameWithOwner":"example-org/alpha","defaultBranchRef":{"name":"main","target":{"committedDate":"2026-08-30T10:00:00Z","statusCheckRollup":{"state":"SUCCESS"}}},"pullRequests":{"totalCount":1,"nodes":[{"number":12,"title":"Add login \"flow\"","headRefName":"feat/login","updatedAt":"2026-09-01T09:30:00Z","isDraft":false}]},"refs":{"totalCount":2,"nodes":[{"name":"main","target":{"committedDate":"2026-08-30T10:00:00Z"}},{"name":"feat/login","target":{"committedDate":"2026-07-01T00:00:00Z"}}]}},"r1":null,"r2":{"nameWithOwner":"example-org/gamma","defaultBranchRef":null,"pullRequests":{"totalCount":0,"nodes":[]},"refs":{"totalCount":0,"nodes":[]}}},"errors":[{"type":"NOT_FOUND","path":["r1"],"locations":[{"line":1,"column":80}],"message":"Could not resolve to a Repository with the name 'example-org/beta'."}]}
   ```
   （整形して置いてよい。`r2` は「既定枝が無い＝空のリポ」の形）
3. **Day**: `from_unix_seconds(0)` = 1970-01-01・`86_399` → 同日・`86_400` → 翌日。`parse_iso8601` の受理と拒否（上記の全ケース＋`2026-09-02T12:00:00+09:00` は None）。
   既知の日付との照合（`2000-03-01` = 11017 日、`2026-09-02` = 20698 日。**自分でも `date -d 2026-09-02 +%s` で割って確かめること**）。`days_since` の順序（逆は None）
4. **remote URL**: 上の表の全行＋末尾改行・末尾 `/`・`.git/`・`GithubSlug::new` の拒否ケース

## 3. 完了条件

- `make check` が緑（fmt / clippy×2 / test×2 / conformance / coverage / deny / doc / build 全部）。**個別コマンドで済ませず、最後に必ず `make check` を通す**
- `cargo test -p fleet-top-core` のテスト件数を報告する
- **コミットはしない**（設計リナがレビューして commit する）。`git status --short` で変更一覧を報告する
- 報告に含める: 変更ファイル一覧・テスト件数・`make check` の末尾出力・`#[expect]` を書いたなら**全部列挙**し理由を添える・
  規約で詰まった箇所とどう解いたか・仕様で曖昧だった点と自分の解釈・満たせなかった点

## 4. 🔴 やってはいけないこと

- `Cargo.toml`（root）の `[workspace.lints]`・`clippy.toml`・`Makefile`・`deny.toml`・`docs/coding-rules.md`・`xtask/` を**変更しない**。
  通らないなら設計を変える。それでも通らないなら**理由を書いて止めて報告する**（緩めない・地雷 4）
- `#[expect]` は最小スコープ・`reason = "<規則 ID>: <理由>"` の形のみ。規則 ID は `docs/coding-rules.md` に実在するもの
- `scopegrep` / `scopegrep-core` を**触らない**（ARC-001。コードを共有しない。コピーは可）
- `_work/` 由来のデータを fixture にしない（地雷 5）。fixture は架空データ
- `/home/xi/docker/xi-tools` 以外のリポジトリを変更しない
- `README.md` / `README.ja.md` / `CHANGELOG.md` を触らない（別の作業指示）
- 時間がかかっても仕様を勝手に狭めない。狭めるなら報告に明記する

## 5. ヒント（実測済みの摩擦）

- 添字 `v[i]` / `s[a..b]` は forbid。`get(i)` / `get(a..b)` / `chars()` / `split_once` / `strip_prefix` / `strip_suffix` を使う
- `as` は forbid。`u32::try_from(n)` / `u64::from(x)` / `char::from_u32`
- 数値リテラルは `0_usize` / `128_usize` / `86_400_u64` のように型を付ける
- 関数は 60 行・認知的複雑度 10・ネスト 4・引数 4 まで。JSON パーサは **`Parser` 構造体（残りの入力とオフセット）にメソッドを分ける**と収まる（`scopegrep-core/src/scanner.rs` の形）
- `clippy::pedantic` が deny なので `must_use_candidate`・`missing_errors_doc`・`module_name_repetitions`・`missing_panics_doc` 等も落ちる。最初に小さく書いて `make lint` を早めに回す
- `no_std` では `String` / `Vec` / `format!` / `ToString` は `alloc::` から取る。`core::error::Error` は使える（toolchain 1.98）
- `shadow_unrelated` が deny。`let value = ...; let value = ...;` の再束縛は関係があるときだけ
- `f64` の `FromStr` は `core` にある（`"1.5".parse::<f64>()`）。`no_std` でも使える
- xtask の CNF-007 は `-core` で終わるクレートの `lib.rs` 先頭属性に `#![no_std]` を要求する。crate doc（`//!`）の後・`extern crate alloc;` の前に置く
