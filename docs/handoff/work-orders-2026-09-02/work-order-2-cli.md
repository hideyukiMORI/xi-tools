# 作業指示 2 — `scopegrep` バイナリ（引数・走査・出力・統合テスト）

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

前提: 作業指示 1 で `scopegrep-core` が入っている（`git log` と `scopegrep-core/src/lib.rs` で確認）。

## 0. 最初に読む（この順）

1. `/home/xi/docker/xi-tools/CLAUDE.md`
2. `/home/xi/docker/xi-tools/docs/design/scopegrep.md` — **「CLI」「出力」「終了コード」「テスト」節が今回の正本**
3. `/home/xi/docker/xi-tools/docs/coding-rules.md` — 特に RS-014（出力は1箇所）・RS-015（環境に触るのは配線点）・RS-005（`process::exit` 禁止）
4. `/home/xi/docker/xi-tools/scopegrep-core/src/lib.rs` と公開 API（`parse` / `Document::search` / `Hit` / `ScopePath` / `ParseError`）
5. `/home/xi/docker/xi-tools/xtask/src/main.rs` — 終了コードを `main` から返す書き方の実例

## 1. 作るもの

`scopegrep/` を足場から本実装にする。**依存は `scopegrep-core` だけ**（`path = "../scopegrep-core", version = "0.0.0"` の形。crates.io 単独 publish の余地を潰さない）。

### モジュール構成（型ごと1ファイル・CNF-003）

| ファイル | 役割 |
| --- | --- |
| `src/main.rs` | 配線点。`std::env::args` を読むのは**ここだけ**（RS-015）。`fn main() -> ExitCode` の形で終了コードを返す（`process::exit` は forbid） |
| `src/cli.rs`（＋型ファイル） | 引数 → `Options { needle, paths, format }`。手書き。`--json` / `--` / `-h` `--help` / `-V` `--version`（version は `env!("CARGO_PKG_VERSION")`） |
| `src/walk.rs` | パス列 → 読むファイル列。ファイルは拡張子を問わず、ディレクトリは再帰して `.yml`/`.yaml` のみ。**パスのバイト順で決定的**。`.git` ディレクトリは飛ばす。ディレクトリのシンボリックリンクは辿らない |
| `src/output.rs` | **`print!`/`println!`/`eprint!`/`eprintln!` を書いてよい唯一のモジュール**（RS-014）。人向け1行・JSON Lines・エラー行。`#[expect(clippy::print_stdout, reason = "RS-014: ...")]` 等は**このモジュールの関数に**付ける |
| `src/run.rs` 等 | 走査→読み込み→`parse`→`search`→出力の流れ。I/O エラーと `ParseError` はファイル単位で報告して続行 |

`main.rs` の足場（`EXIT_NOT_IMPLEMENTED`・`scaffold_builds` テスト・`main` の `#[expect(clippy::print_stderr)]`）は**消す**。
出力の `#[expect]` は `output.rs` へ移る。移し忘れは `unfulfilled_lint_expectations` が落とす。

### 出力仕様（設計メモと完全一致させる）

人向け:
```
<file>:<line>: <path> = <value>
```
JSON（キー順固定・7キー・`label` 無しは `null`・RFC 8259 エスケープを手書き）:
```
{"file":"…","line":33,"column":18,"pointer":"/jobs/frontend-check/steps/3/if","path":"jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if","label":"Audit (fail on high/critical)","value":"${{ !cancelled() }}"}
```
エラー（標準エラー）:
```
scopegrep: <file>:<line>: <message>          ← ParseError（message は ParseError の Display）
scopegrep: <file>: <io error>                 ← 読めない
scopegrep: usage: scopegrep [--json] <needle> <path>...   ← 引数誤り（終了 2）
```
`<file>` は与えられたパスをそのまま／ディレクトリ再帰なら `join` した相対パス。正規化しない。

終了コード: 0 = ヒットあり / 1 = ヒットなし / 2 = エラーあり（ヒットがあっても 2）。

### 順序

ヒットは「ファイル（走査順）→ 行 → 列」。ファイルは読んだ順に出力してよい（走査順が決定的なので結果も決定的）。

## 2. テスト（QLT-007）

`scopegrep/tests/cli.rs` にバイナリを起動する統合テスト（`env!("CARGO_BIN_EXE_scopegrep")` + `std::process::Command`）。
fixture は `scopegrep-core/testdata/workflow-with-comment.yml`（`concat!(env!("CARGO_MANIFEST_DIR"), "/../scopegrep-core/testdata/…")`）。
CLI 固有の fixture（ディレクトリ走査・エラー用の不正 YAML・拡張子違い）は `scopegrep/testdata/` に**架空データで**新設してよい。

必須:

1. fixture に対する人向け出力の**完全一致**（2行）と終了コード 0
2. `--json` の**完全一致**（2行・キー順・`column` の値を `grep -n`/手計算で確認して書く）
3. ヒット無し → 標準出力が空・終了コード 1
4. 部分集合の外の YAML（例: アンカー）→ 標準エラーに `scopegrep: <file>:<line>: …`、終了コード 2。**別の正常ファイルのヒットは出ていて、それでも 2**
5. 存在しないパス → 終了コード 2
6. 引数なし／needle だけ → usage・終了コード 2。`--help` は 0、`--version` は 0
7. ディレクトリ走査: `.yml` と `.yaml` は読む・`.txt` は読まない・順序がバイト順・`.git/` 配下は読まない（テスト内で一時ディレクトリを作る。`std::env::temp_dir()` 配下に一意な名前で作り、終わったら消す）
8. `cli.rs` の引数解析の単体テスト（`--` の扱い・`--json` の位置）

## 3. 完了条件

- `make check` 緑。最後に必ず `make check`
- `cargo run -p scopegrep -- 'cancelled()' scopegrep-core/testdata/` の実出力を報告に貼る
- **コミットしない。** `git status --short` を報告
- 報告に含める: 変更ファイル一覧・テスト件数・`make check` 末尾・`#[expect]` の全列挙と理由・曖昧だった仕様と解釈・満たせなかった点（無ければ「無し」）

## 4. 🔴 やってはいけないこと

- root `Cargo.toml` の `[workspace.lints]`・`clippy.toml`・`Makefile`・`docs/coding-rules.md` を変えない（地雷4）
- `scopegrep-core` の公開 API を変えない。足りなければ**報告で要求する**（設計リナが判断する）。ただし core の**バグ**（テストが示せるもの）は直してよい。直したら報告に書く
- 依存を足さない（`clap` / `serde_json` / `walkdir` 等は ADR 事項）
- README は触らない（次回）
- `_work/` 由来のデータを fixture にしない

## 6. 追記（2026-09-02・core レビュー後の裁定）— 最初にやる core の小修正

作業指示 1 のレビューで設計リナが裁定した2点。**CLI に入る前に core で直し、テストを更新すること。**

1. **ラベルはクォートを1枚外す**（キーと同じ扱い）。`name: "Build"` → ラベル `Build`、`name: 'A ''b'''` → `A ''b'`（エスケープは解除しない）。
   現状はラベルが値の原文（クォート付き）なので変更する。`ScopePath::label()`・`Display`・JSON の `label` すべてに効く。テストを足す（クォート付き name のケース）
2. **`Hit::column()` は「一致の先頭」で確定**（現行実装どおり・fixture では 33/46 行目とも 18）。設計メモの JSON 例は 18 に直してある。core は変更不要。CLI の `--json` テストの期待値は 18

設計メモ `docs/design/scopegrep.md` の D-1 節（ラベルの規則）と公開 API 節にこの裁定を反映済み。
