# 作業指示 1 — `scopegrep-core` クレート（no_std スキャナ＋検索）

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

## 0. 最初に読む（この順・飛ばさない）

1. `/home/xi/docker/xi-tools/CLAUDE.md` — 第一目的と地雷6件
2. `/home/xi/docker/xi-tools/docs/design/scopegrep.md` — **今回の仕様の正本**。D-1〜D-4・アーキテクチャ・公開 API・YAML 部分集合・検索の意味
3. `/home/xi/docker/xi-tools/docs/coding-rules.md` — 規約。特に RS-001/002/003/005/007/008/011/012/013/016/018、ARC-003、QLT-006
4. `/home/xi/docker/xi-tools/xtask/src/*.rs` — **この規約下で書かれたコードの実例**。書き方（`get()` / `try_from` / 型付きリテラル / `#[expect]` の形）はここに倣う
5. `/home/xi/docker/xi-tools/Cargo.toml` / `clippy.toml` / `Makefile`

## 1. 作るもの

workspace に **`scopegrep-core`** クレート（lib）を足す。`/home/xi/docker/xi-tools/scopegrep-core/`。

- `#![no_std]` + `extern crate alloc`。**依存 0**（`[dependencies]` は空）
- `Cargo.toml` は `scopegrep/Cargo.toml` に倣う（`edition.workspace` 等・`[lints] workspace = true`・version `0.0.0`・description は日本語で1行）。`readme` は書かない（core は README を持たない）
- root `Cargo.toml` の `members` に足す。`Cargo.lock` は `cargo` に更新させてコミット対象にする
- fixture `scopegrep/testdata/workflow-with-comment.yml` を **`scopegrep-core/testdata/` へ `git mv`** する（CLI の統合テストは次回、ここを参照する）

### 公開 API（設計メモの「`scopegrep-core` の公開 API」節と一致させる）

```rust
pub fn parse(source: &str) -> Result<Document, ParseError>;
impl Document { pub fn search(&self, needle: &str) -> Vec<Hit>; }
impl Hit { path() -> &ScopePath; line() -> LineNumber; column() -> Column; value() -> &str }
impl ScopePath { pointer() -> String; label() -> Option<&str> }   // + Display（人向け形式）
impl LineNumber { pub fn get(self) -> u32 }  impl Column { pub fn get(self) -> u32 }
impl ParseError { line() -> LineNumber; kind() -> ParseErrorKind }  // + Display + core::error::Error
pub enum ParseErrorKind { Unsupported(...), Malformed(...) }  // 中身は設計メモの表を表せる enum に
```

- `Document` の内部表現は非公開。木で持つか平坦な表で持つかは任せる（**`HashMap` 禁止・順序は決定的**）
- 型ごとに1ファイル（CNF-003: 桁 0 の `struct`/`enum`/`trait`/`type` 宣言は1ファイルに1つ）。`lib.rs` は `mod` 宣言と crate doc だけ
- モジュール名に `utils`/`helpers`/`common`/`misc` を使わない。型名の語尾に `Manager`/`Helper`/`Util(s)`/`Common` を使わない
- `Default` を実装・derive しない。`mod.rs` を作らない。`pub use` を書かない
- 公開項目には doc コメント（日本語でよい）。`# Errors` 節（`missing_errors_doc`）も要る

### 振る舞い

設計メモの「対応する YAML の部分集合（v1）」「検索の意味」を**そのまま**実装する。要点:

- コメント判定: 行頭（空白のみの後）の `#`、または**空白の直後**の `#`。クォート内の `#` はコメントではない。ブロックスカラーの内容行に `#` があってもコメントではない
- 列（`Column`）は 1 始まりの**文字数**（`char` の数。バイトではない）
- 値は**原文のまま**（クォートの中身をエスケープ解除しない。プレーンスカラーは末尾空白と行末コメントを除く）
- ブロックスカラーは**内容の各行を別のスカラー行**として持つ。ヒットの `line()` はその行、`value()` はその行の内容（先頭のブロックインデントを除いた原文）
- ラベル: シーケンス要素がマッピングで `name` キーの値が1行スカラーなら、そのスカラーの原文がラベル
- 部分集合の外は `ParseError`。**黙って誤読しない。** 種別と行番号を必ず持つ
- 順序: ヒットは行→列の順

### 人向けパス表示（`Display for ScopePath`）— 完全一致で試験する

```
jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .if
jobs.e2e.steps[2] "Upload Playwright report" .if
jobs.e2e.steps[0].uses            ← ラベルが無い要素
on.pull_request                   ← 空の値は検索に出ないが、パス表示の規則は同じ
"weird key".x                     ← キーに [A-Za-z0-9_-] 以外があれば "…" で囲む。ラベル内の " と \ は \ でエスケープ
```

JSON Pointer（`pointer()`）: `/jobs/frontend-check/steps/3/if`。`~`→`~0`、`/`→`~1`。ラベルは含めない。

## 2. テスト（QLT-007。これが無いと受け取らない）

`#[cfg(test)]` モジュール内では `extern crate std;` を使ってよい（`clippy.toml` がテストの `unwrap`/添字を免除している）。

必須:

1. **fixture 検証**: `include_str!("../testdata/workflow-with-comment.yml")` を `parse` → `search("cancelled()")` の結果が**ちょうど2件**で、`line()` が 35 と 47（`grep -n` で確認せよ）、`pointer()` が `/jobs/frontend-check/steps/3/if` と `/jobs/e2e/steps/2/if`、`Display` が上の完全一致、`label()` が `Some("Audit (fail on high/critical)")` / `Some("Upload Playwright report")` であること。**コメント内（29〜30行目）が無いこと**を明示的に検証する
2. 部分集合の**各構文**に1テスト以上（コメント各形・クォート内 `#`・ブロックスカラーの `#`・同じ桁のシーケンス・`- key: v` 始まりのマッピング・1行フロー・空の値・`---`・`\r\n`・BOM）
3. **エラーにする構文が実際にエラーになる**テスト（アンカー・エイリアス・タグ・マージキー・継続行・複数行フロー・2つ目の `---`・複合キー・タブインデント・浅すぎる子）。`kind()` と `line()` を検証する
4. `pointer()` のエスケープ（`~` と `/` を含むキー）、`Display` のクォート規則
5. 列が文字数であること（日本語を含む値で検証）

## 3. 完了条件

- `make check` が緑（fmt / clippy / test / conformance / doc / build 全部）。**個別コマンドで済ませず、最後に必ず `make check` を通す**
- `scopegrep-core/src/lib.rs` 冒頭の crate doc に「何を読めて何を読めないか」への案内（設計メモへのリンク）がある
- **コミットはしない**（設計リナがレビューして commit する）。`git status` で変更一覧を報告する
- 報告に含める: 変更ファイル一覧・テスト件数・`make check` の末尾出力・**規約で詰まった箇所と、どう解いたか**（`#[expect]` を書いたなら全部列挙し理由を添える）・仕様で曖昧だった点と自分の解釈

## 4. 🔴 やってはいけないこと

- `Cargo.toml`（root）の `[workspace.lints]`・`clippy.toml`・`Makefile`・`docs/coding-rules.md` を**変更しない**。通らないなら設計を変える。それでも通らないなら**理由を書いて止めて報告する**（緩めない・地雷4）
- `#[expect]` は最小スコープ・`reason = "<規則 ID>: <理由>"` の形のみ。規則 ID は `docs/coding-rules.md` に実在するもの
- `_work/` 由来のデータを fixture にしない（地雷5）。fixture は架空データ
- `scopegrep/src/main.rs` は**触らない**（次回の作業）
- `/home/xi/docker/xi-tools` 以外のリポジトリを変更しない
- 時間がかかっても仕様を勝手に狭めない。狭めるなら報告に明記する

## 5. ヒント（実測済みの摩擦）

- 添字 `v[i]` / `s[a..b]` は forbid。`get(i)` / `get(a..b)` を使う
- `as` は forbid。`u32::try_from(n)` / `usize::from(x)`
- 数値リテラルは `0_usize` のように型を付ける
- 関数は 60 行・認知的複雑度 10・ネスト 4・引数 4 まで。スキャナは**状態を小さな型に分けて**関数を短く保つ
- `clippy::pedantic` が deny なので `must_use_candidate`・`missing_errors_doc`・`module_name_repetitions` 等も落ちる。最初に小さく書いて `make lint` を早めに回す
- `no_std` では `String`/`Vec`/`BTreeMap`/`format!` は `alloc::` から取る。`core::error::Error` は 1.81 以降で使える（toolchain は 1.98）
