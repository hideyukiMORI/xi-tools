# 作業指示 6 — 毎日使える形にする: `--scope`・`-i`・パス省略・依存ディレクトリの除外

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

施主の要求は「便利に使えるレベルまで」。設計リナが実測（下記）で 4 点に絞った。**依存は増やさない。**

## 0. 最初に読む

1. `/home/xi/docker/xi-tools/CLAUDE.md`
2. `/home/xi/docker/xi-tools/docs/design/scopegrep.md` — 公開 API・CLI・出力の節（**本指示の内容は設計リナが後で反映する。編集不可**）
3. `/home/xi/docker/xi-tools/scopegrep-core/src/{lib,document,search_scope,hit,scope_path,segment}.rs`
4. `/home/xi/docker/xi-tools/scopegrep/src/{cli,options,argument,walk,run,render}.rs`・`tests/{cli,readme}.rs`
5. `/home/xi/docker/xi-tools/docs/daily/2026-09-02.md` の「規約で詰まった箇所」

作業ディレクトリ `/home/xi/docker/xi-tools`、ブランチ `feat/scopegrep-usable`（クリーン。切り替えない・コミットしない）。

## 1. 実測（設計リナ・2026-09-02）

| | 数 |
| --- | --- |
| 自前の `.yml`/`.yaml`（`node_modules` 等を除く） | 188 |
| `node_modules` 配下の `.yml`/`.yaml` | 3,206 |
| `vendor` 配下 | 3,837 |
| `.venv` 配下 | 1 |
| `target` 配下 | 0 |
| `~/docker` 全体を v1.1 で走査 | 5.6 秒・エラー 30 件（ほぼ依存ディレクトリ由来） |

## 2. 作るもの

### 2-a. `--scope <pattern>` — 構造で絞る（本命）

```
scopegrep --scope '/jobs/*/steps/*/if' '' .github/workflows/     # 全ステップの if 条件を列挙
scopegrep --scope '/services/**/image' 'postgres' compose.yaml    # どの深さでも image キー
```

- パターンは **JSON Pointer の形**（`/` 区切り・先頭 `/` 必須）。セグメントは `*`（ちょうど 1 セグメント）、`**`（0 個以上）、それ以外はリテラル。
  リテラルはポインタのエスケープ（`~0` `~1`）を解除した後の**生のキー／索引文字列**と完全一致（`*` を部分一致のグロブにはしない。**一つの意味**）
- ヒットの `pointer()` に対して**全体一致**（前方一致ではない）。コメントヒット（`--comments`）も同じ規則で、その所属ポインタに当てる。ルートのコメントはポインタ `""` なので `/**` にだけ当たる
- 先頭が `/` でない・空・`//` を含む → usage エラー（終了 2・メッセージに理由）
- `--scope` を 2 回書いたら usage エラー（後勝ちにしない）
- **core に置く**: `scope_pattern.rs`（`ScopePattern::parse(&str) -> Result<Self, ScopePatternError>`・`matches(&ScopePath) -> bool`）。マッチは `**` を含むので再帰か DP で書く（複雑度 10 以内に割る）

### 2-b. `-i` / `--ignore-case`

- 大文字小文字を無視して一致。**列は原文の一致位置**のまま（`to_lowercase` した文字列上の index を使わない。`ß`→`ss` のように長さが変わる文字で列がずれる）。
  文字ごとに `a.to_lowercase().eq(b.to_lowercase())` で比べる形にする（`char::to_lowercase` はイテレータ）
- core: 検索条件を 1 つの型に束ねる。**`Document::search(&self, query: &Query) -> Vec<Hit>`** に変える
  ```rust
  pub struct Query { needle, case: CaseMatch, kinds: SearchScope, within: Option<ScopePattern> }  // フィールド非公開
  impl Query { pub fn new(needle: &str) -> Self; pub fn ignoring_case(self) -> Self; pub fn including_comments(self) -> Self; pub fn within(self, pattern: ScopePattern) -> Self; }
  pub enum CaseMatch { Exact, Fold }   // case_match.rs（RS-002・閉じた選択肢）
  ```
  既存の `search(needle, SearchScope)` は消す（呼び出しを全部 `Query` に直す。doctest も）。**既存テストの期待値は 1 つも変えない**

### 2-c. パス省略時は `.`

- `scopegrep <needle>` だけで `.` を再帰する。**表示は `./` を付けない**（`.github/workflows/ci.yml`）。
  明示的に `.` を渡したときは従来どおり `./` 付き（`grep -rn x .` と同じ。与えたパスをそのまま使う規則を崩さない）
- usage 文字列: `scopegrep [-i] [--json] [--comments] [--scope <pattern>] <needle> [<path>...]`

### 2-d. 依存ディレクトリを走査から外す

- ディレクトリ再帰のとき、名前が **`.git` `node_modules` `vendor` `target` `.venv`** のディレクトリに入らない（固定リスト・旗で変えられない。`dist` は施主の実フォルダ名と衝突するので**入れない**）
- **コマンドラインで名指しされたパス**（ファイルでもディレクトリでも）は除外しない（`scopegrep x node_modules/foo/` は読む）
- リストは `walk.rs` の定数 1 箇所に置き、doc コメントに上の実測（3,206 / 3,837）を書く

## 3. テスト（QLT-007）

- core: `ScopePattern` の解析（正常・先頭 `/` 無し・空セグメント）、マッチ（`*` は 1 つ・`**` は 0 個以上・全体一致・`~0` `~1` の解除・ルート `""` に `/**` だけが当たる）、`-i` の一致と**原文の列**（日本語・`ß` を含むケース）
- CLI: `--scope` の人向け完全一致（fixture で `/jobs/*/steps/*/if` と `''` → 2 行）、不正パターンの usage・終了 2、`-i` の一致、パス省略で `./` が付かないこと（一時ディレクトリで cwd を変えて起動）、除外リスト（一時ディレクトリに `node_modules/x.yml` を置いて読まれない・名指しなら読まれる）
- README: 「使い方」の節を足し、`--scope` の例を **1 ブロック実行して貼る**（`tests/readme.rs` が照合する）。上の実測の数字を除外リストの説明に **1 文**入れる（数字は本指示の表から。盛らない）

## 4. 完了条件

- `make check` 緑。**コミットしない**。`git status --short` を報告
- 報告: 変更ファイル・テスト件数・`make check` 末尾・`--scope` と `-i` とパス省略の実出力・`#[expect]` の全列挙・曖昧だった点と解釈・満たせなかった点（無ければ「無し」）

## 5. 🔴 やってはいけないこと

- 依存を足さない（`regex` / `glob` / `ignore` / `clap` は ADR 事項）。`.gitignore` の解釈を実装しない（固定リストが今回の「一つの手段」）
- ゲート設定を変えない。`COVERAGE_MIN_LINES` を下げない
- 設計メモを編集しない。`_work/` 由来のデータを fixture にしない
- 既定の出力形式を変えない（`<file>:<line>: <path> = <value>`）
