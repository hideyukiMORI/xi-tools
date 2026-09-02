# 作業指示 7 — 正規表現は opt-in の feature（ADR 0002）＋ `cargo-deny`

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

前提: 作業指示 6（`--scope` / `-i` / パス省略 / 除外）が入っている。ブランチ `feat/scopegrep-usable`。切り替えない・コミットしない。

## 0. 最初に読む

1. `/home/xi/docker/xi-tools/docs/adr/0002-regex-is-an-opt-in-feature.md` — **今回の正本。決定 1〜5 をそのまま実装する**
2. `/home/xi/docker/xi-tools/CLAUDE.md`・`docs/coding-rules.md`（ARC-004・QLT-003・QLT-007）
3. `/home/xi/docker/xi-tools/scopegrep-core/src/{lib,query,document,case_match}.rs`（作業指示 6 で入った `Query`）
4. `/home/xi/docker/xi-tools/scopegrep/src/{cli,options,run}.rs`・`Cargo.toml`・`Makefile`・`.github/workflows/ci.yml`
5. `/home/xi/docker/xi-tools/docs/quality/gate-proofs.md`（証明の書き方）

## 1. 作るもの

### 1-a. core: 一致の判定を trait で受け取る（依存 0・`no_std` のまま）

```rust
/// 1 行のスカラーテキストに対する一致判定。`find` は一致の先頭の文字位置（0 始まり・char 数）を返す。
pub trait Matcher { fn find(&self, text: &str) -> Option<usize>; }     // matcher.rs
```

- 固定文字列（大文字小文字あり／なし）の実装は core に置く（既存の検索コードを `Matcher` 実装に移す。**振る舞いと列は 1 つも変えない**）
- `Query` は `needle: &str` の代わりに `Box<dyn Matcher>`（または generics）を受ける。**`Query::new(needle)` の既存の使い勝手は残す**
  （固定文字列の `Matcher` を内部で作る）。新たに `Query::with_matcher(Box<dyn Matcher>)` を足す
- `dyn Matcher` は「汎用データバッグ」ではない（意味のある trait）。CNF-002 の `dyn Any` 検出に当たらないことを確認

### 1-b. bin: feature `regex`

`scopegrep/Cargo.toml`:
```toml
[features]
regex = ["dep:regex"]

[dependencies]
scopegrep-core = { path = "../scopegrep-core", version = "0.0.0" }
regex = { version = "1", optional = true, default-features = false, features = ["std", "unicode"] }
```
- 版は crates.io の現行 1.x を **`=` ではなく `1`** で書く（`wildcard_dependencies` は `*` だけを禁じる）。`Cargo.lock` が固定する
- `regex_matcher.rs`（`#[cfg(feature = "regex")]`）に `Matcher` 実装。`-i` は `RegexBuilder::case_insensitive(true)`。
  `find` は `Regex::find` の byte offset を **char 位置に変換**して返す（既存の列の意味＝1 始まりの文字数と揃える）
- CLI: `-e <pattern>` / `--regex <pattern>`。**`<needle>` 位置引数と排他**（両方あれば usage エラー）。
  feature 無しビルドで `-e` を打ったら終了 2・標準エラーに
  `scopegrep: この binary は正規表現なしでビルドされている（cargo install --features regex）`。**黙って固定文字列として扱わない**
- 不正な正規表現 → 終了 2・`scopegrep: 正規表現が不正: <regex の Display>`
- usage: `scopegrep [-i] [--json] [--comments] [--scope <pattern>] (<needle> | -e <regex>) [<path>...]`
- `--version` の出力に feature の有無を出す: `scopegrep 0.0.0 (regex: on)` / `(regex: off)`。テストで両方固定

### 1-c. `cargo-deny`

- `deny.toml` をリポジトリ root に置く。方針は ADR 0002 決定 4:
  - `[licenses]` `allow = ["MIT", "Apache-2.0", "Unicode-3.0", "BSD-2-Clause", "BSD-3-Clause"]`（`regex` 系の実ライセンスを `cargo deny list` で確認して**実際に要るものだけ**書く）
  - `[advisories]` 脆弱性・unmaintained・yanked を失敗に
  - `[bans]` `multiple-versions = "deny"`、`wildcards = "deny"`
  - `[sources]` crates.io 以外を拒否
- `Makefile` に `deny` ターゲット（`cargo deny --locked check`）を足し、`check` に含める。`cargo-deny` が無いときは coverage と同じ形で終了 2 とメッセージ
- CI に導入ステップ（`taiki-e/install-action@cargo-deny`）。検査そのものは `Makefile`（QLT-003）
- **発火の証明**: `deny.toml` の allow から `MIT` を一時的に消して `make deny` → 実出力を `gate-proofs.md` に（P-18）。戻す

### 1-d. `make check` は両構成で回す

- `Makefile` の `test` と `lint` を feature あり・なしの両方で走らせる（`--features regex` 付きをもう 1 回）。
  `coverage` は feature あり（両方の経路を測る）。`build` は既定（依存 0 の binary が成果物）
- `lint`/`test` の 2 回目は既存ターゲットの中で回す。**CI 側に新しい検査を書かない**

### 1-e. 文書（実装の事実だけ）

- `docs/coding-rules.md` ARC-004: 「現在の依存は 0」→「**既定ビルドの依存は 0**。opt-in の `regex` は ADR 0002」、
  planned（`cargo-deny`）→ **active**（`make deny`・証明 P-18）。第7節の `cargo-deny` 不採用行を消す（採用したので）
- README: 「依存は 0」→「既定ビルドの依存は 0」。`--regex` の使い方を 1 段落と、`cargo install --path scopegrep --features regex` の 1 行。
  `--regex` の例を **feature ありで実行して**貼る場合、`tests/readme.rs` は feature 無しビルドでその例を**スキップではなく失敗**させてしまうので、
  README の `-e` の例は **`console` ブロックに入れない**（本文に書く）か、`readme.rs` が `(regex: off)` のとき `-e` 行を「未検証」として数え、
  少なくとも 1 つは検証済みであることを要求する形にする。**どちらにしたかを報告する**
- `.github/pull_request_template.md` の planned 一覧から `cargo-deny` を外す

## 2. テスト

- core: `Matcher` 経由でも既存テストが 1 つも変わらないこと。固定文字列の `Matcher` の単体テスト
- bin（feature あり）: `-e 'cancel+ed\(\)'` の人向け完全一致、`-i -e`、`--scope` との併用、不正な正規表現の終了 2、`--version` の `(regex: on)`
- bin（feature なし）: `-e` の終了 2 とメッセージ、`--version` の `(regex: off)`。`#[cfg(feature = "regex")]` / `#[cfg(not(feature = "regex"))]` でテストを分ける
- `deny`: 発火の証明（上）

## 3. 完了条件

- `make check` 緑（両構成）。**コミットしない**。`git status --short` を報告
- 報告: 変更ファイル・テスト件数（両構成）・`make check` 末尾・`cargo deny list` の実出力（ライセンス一覧）・P-18 の実出力・
  README の `-e` 例の扱い・`#[expect]` の全列挙・曖昧だった点・満たせなかった点

## 4. 🔴 やってはいけないこと

- core に `regex` を入れない。core の `no_std`・依存 0 を崩さない
- ゲート設定を緩めない。`COVERAGE_MIN_LINES` を下げない（feature ありで測るので下がらないはず。下がったら理由を報告して止める）
- `regex` 以外の依存を足さない。設計メモ・ADR を編集しない（食い違いは報告）
- `deny.toml` の allow に**使っていないライセンス**を書かない
