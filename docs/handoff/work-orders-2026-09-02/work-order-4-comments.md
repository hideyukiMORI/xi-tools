# 作業指示 4 — `--comments`: コメント内のヒットを「別枠」で返す

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

## 0. 最初に読む（この順）

1. `/home/xi/docker/xi-tools/CLAUDE.md`
2. `/home/xi/docker/xi-tools/docs/design/scopegrep.md` — 仕様の正本。**本指示の内容を、設計リナが後で同メモへ反映する。実装リナはメモを編集しない**
3. `/home/xi/docker/xi-tools/docs/coding-rules.md`
4. `/home/xi/docker/xi-tools/scopegrep-core/src/{lib,scanner,scalar_value,scope_path,hit,document}.rs`
5. `/home/xi/docker/xi-tools/scopegrep/src/{cli,options,render,output,run}.rs` と `tests/{cli,readme}.rs`
6. `/home/xi/docker/xi-tools/docs/daily/2026-09-02.md` の「規約で詰まった箇所」（解き方の一覧）

作業ディレクトリ: `/home/xi/docker/xi-tools`、ブランチ `feat/scopegrep-comments`（切り替えない・コミットしない）。

## 1. 何を足すか（設計）

`scopegrep` の存在理由は「コメント内の一致を設定値と区別できる」ことである。今は区別した結果コメントを**黙って捨てる**。
`--comments` を付けたときだけ、コメント内のヒットを**「コメントである」と明示して**返す。既定の挙動は変えない。

### 意味

- **コメント**とは、スキャナが既にコメントと判定しているもの: 行全体のコメント、値の後ろの行末コメント。
  ブロックスカラーの内容行の `#` はコメントではない（従来どおり値）
- コメントヒットの**所属（path）は「そのコメントがどの入れ子の中に書かれたか」**であり、「誰の説明か」ではない。
  🔴 ここを推測しない（tree-sitter が「直前の兄弟に付ける」で誤る箇所。設計メモ「D-2 実測」）。規則は機械的に:
  - 行全体のコメント: **その行のインデントで開いている最も内側のコンテナ**のパス。
    fixture の 29〜30 行目（インデント 6・`steps` の要素の桁）→ `jobs.frontend-check.steps`。
    32 行目 `        # 2) 欠陥。…`（インデント 8・`steps[3]` のキーの桁）→ `jobs.frontend-check.steps[3]`。
    4 行目（インデント 0・何も開いていない）→ **ルート**
  - 行末コメント（`key: value # note`）: **その値のパス**
  - コンテナの判定に迷う境界（コンテナのキーの桁より浅いが親より深い等）は「**より浅い方＝外側**」に付ける。理由を doc コメントに書く
- ルートのパス: JSON Pointer は **空文字列 `""`**（RFC 6901 で「文書全体」を指す正規の表現）。人向け表示も空
- 順序: 値ヒットとコメントヒットを**行→列でマージ**して返す（ファイル内で行番号順）
- `column` は従来どおり一致の先頭（1 始まり・文字数）
- `value` はコメント行なら **`#` から行末までの原文**（行頭の空白は含めない。行末コメントも `#` から）

### `scopegrep-core` の API 変更（最小）

```rust
/// 何を探すか（閉じた選択肢・RS-002）
pub enum SearchScope { Values, ValuesAndComments }   // search_scope.rs
/// ヒットの種別
pub enum HitKind { Value, Comment }                   // hit_kind.rs

impl Document {
    pub fn search(&self, needle: &str, scope: SearchScope) -> Vec<Hit>;   // 引数を1つ足す（既存の呼び出しは Values）
}
impl Hit { pub fn kind(&self) -> HitKind; }           // 追加
```

- `Document` の内部にコメント行の記録（行・列・原文・所属パス）を足す。表現は任せる（`HashMap` 禁止・順序決定的）
- 既存テストは `SearchScope::Values` を渡す形に直す。**既存の期待値は 1 つも変えない**
- `lib.rs` の doctest も追随

### CLI

- 旗 `--comments`。`--json` と同様に `--` より前ならどこでも可。usage 文字列に足す: `scopegrep [--json] [--comments] <needle> <path>...`
- 人向け: コメントヒットは `<file>:<line>: <path> #comment = <value>`。ルートは path が空なので `<file>:<line>: #comment = <value>`
  （`#comment` の前に path があるときは空白 1 つで区切る。値ヒットの行は従来どおり変えない）
- JSON: **`kind` キーを 8 番目に足す**（`"kind":"value"` / `"kind":"comment"`）。`--comments` 無しでも常に `"kind":"value"` が出る（キー数は常に 8）。
  🔴 README の `--json` 例と `tests/cli.rs` `tests/readme.rs` の期待値がこれで変わる。**実行して貼り直す**
- 終了コード: コメントヒットもヒットに数える（0）

### fixture に対する期待（実行して確認してから書くこと。以下は設計リナの手計算）

```
$ scopegrep --comments 'cancelled()' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:4: #comment = #    候補パーサは、下の3つの `cancelled()` を **別物として区別できなければならない**。
scopegrep-core/testdata/workflow-with-comment.yml:29: jobs.frontend-check.steps #comment = # 1) 散文。ここに書かれた cancelled() は設定値ではない。
scopegrep-core/testdata/workflow-with-comment.yml:30: jobs.frontend-check.steps #comment = #    !cancelled() を使う理由を説明しているだけで、実行条件ではない。
scopegrep-core/testdata/workflow-with-comment.yml:33: jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .if = ${{ !cancelled() }}
scopegrep-core/testdata/workflow-with-comment.yml:46: jobs.e2e.steps[2] "Upload Playwright report" .if = ${{ !cancelled() }}
```

`grep -n` の 5 行と**同じ 5 行**が、種別付きで返る。これが README に足す新しい例になる。

## 2. テスト（QLT-007）

- core: 行全体コメント（ルート／コンテナ内／要素マッピング内の 3 段）、行末コメント、ブロックスカラー内 `#` はコメントにならない、`Values` では従来どおり出ない、順序のマージ、ルートの pointer が `""`
- CLI: `--comments` の人向け完全一致（上の 5 行）、`--json --comments` の完全一致、`--comments` 無しの既存出力が**変わらない**こと
- README: `--comments` の例を 1 ブロック足し、`tests/readme.rs` が照合することを確認（README の「コメント内の一致は返しません」の段落は「既定では返さず、`--comments` で種別付きで返す」に事実どおり直す。**盛らない**）

## 3. 完了条件

- `make check` 緑（coverage 含む。`cargo-llvm-cov` は導入済み）
- **コミットしない**。`git status --short` を報告
- 報告: 変更ファイル・テスト件数・`make check` 末尾・上の 5 行の実出力・`#[expect]` の全列挙・曖昧だった点と解釈・所属の境界規則で迷ったケース（あれば具体例）・満たせなかった点

## 4. 🔴 やってはいけないこと

- ゲート設定（root `Cargo.toml` lints・`clippy.toml`・`Makefile` の下限・`rust-toolchain.toml`）を変えない。`COVERAGE_MIN_LINES` を下げない
- 「誰の説明か」を推測するロジックを入れない（上の機械的規則のみ）
- 設計メモ `docs/design/scopegrep.md` を編集しない（設計リナが反映する）。`docs/todo/current.md` の「PR #2 のレビューとマージ 🔲 施主」の行は **`✅ 2026-09-02 マージ済み`** に直してよい
- 依存を足さない。`_work/` 由来のデータを fixture にしない
