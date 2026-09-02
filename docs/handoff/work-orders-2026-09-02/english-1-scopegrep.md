# 作業指示 english-1 — `scopegrep` / `scopegrep-core` の利用者向け文言を英語にする

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

## 0. 最初に読む（この順・飛ばさない）

1. `/home/xi/docker/xi-tools/CLAUDE.md` — 第一目的（**public に示す**）と地雷 6 件。特に地雷 2（README の例はテストが照合する）
2. `/home/xi/docker/xi-tools/docs/coding-rules.md` — 規約。今回は文字列しか触らないが、`make check` は全部通す
3. `/home/xi/docker/xi-tools/scopegrep/src/output.rs`・`usage_error.rs`・`cli.rs`・`regex_matcher.rs` — bin 側の文言
4. `/home/xi/docker/xi-tools/scopegrep-core/src/unsupported_syntax.rs`・`malformed_input.rs`・`document.rs`・`scope_pattern_error.rs`・`scope_pattern.rs`・`query.rs`・`column.rs`・`parse_error*.rs`・`fixed_string.rs`・`scanner.rs`・`mapping_entry.rs` — core 側の `Display` 文言
5. `/home/xi/docker/xi-tools/scopegrep/tests/cli.rs`・`tests/readme.rs` — 期待文字列を持つテスト

## 1. 何を、なぜ

README は英語なのに、実行すると `--help` も stderr も日本語が出る。第三者が最初に踏む段差なので、
**利用者の目に触れる文字列**を英語にする。`fleet-top` 側は別の作業指示（english-2）で同時に進めるので、
**両ツールで同じ語を同じ英語にする**（下の用語表）。

### 対象（変える）

- `--help` の全文（見出し `usage:` / `arguments:` / `options:` / `exit status:` 等の**構造は変えない**。中身を英語に）
- 使い方エラー（`UsageError` の `Display`）と `output::usage` が出す文
- stderr に出る全ての文（読めないファイル・部分集合の外・正規表現なしビルド等）
- `scopegrep-core` の公開エラー型の `Display`（`ParseError` / `ParseErrorKind` / `UnsupportedSyntax` / `MalformedInput` / `ScopePatternError` 等）
- `--version` の表示（`(regex: on)` は既に英語。変えない）

### 対象外（変えない）

- コード内のコメント・doc コメント・テスト名・`expect("…")` / `expect_err("…")` / `assert!` のメッセージ（これらは開発者向け。リポの規約で日本語）
- fixture（`testdata/*.yml`）の中身。README の `console` ブロックに出る fixture 由来の日本語（`#comment = # 1) 散文…`）はデータであって文言ではない
- 出力の**形式**（`file:line: scope = value` の形・JSON のキー・終了コード）。**文字列の英語化だけ**で振る舞いを変えない
- `Cargo.toml` の `description`（既に英語）

## 2. 英語の書き方（両ツール共通）

- 1 行メッセージは**文頭小文字・末尾ピリオド無し**（`grep` / `git` / `cargo` の流儀。例: `scopegrep: usage: …`・`fleet-top: alpha: gh not found`）
- `--help` の説明文も文頭小文字・ピリオド無し。複数文になるときだけピリオドで区切る
- ASCII の句読点だけ（全角記号・`——`・`「」` を使わない。引用は `` ` `` か `'`）
- 値は原文のまま埋め込む（`{path}` / `{line}` / `{text}` の位置を変えない）
- 「〜ではない」は `is not …`、「〜が無い」は `missing …` / `no …`、「読めない」は `cannot read …`、「想定外の形」は `unexpected …`

| 日本語 | 英語 |
| --- | --- |
| 使い方 | usage |
| 探す固定文字列 | the fixed string to search for |
| 正規表現 | regular expression |
| 部分集合の外 | outside the supported subset |
| アンカー / エイリアス / タグ / マージキー | anchor / alias / tag / merge key |
| 複数ドキュメント | multiple documents |
| 継続行（複数行プレーンスカラー） | multi-line plain scalar |
| 複合キー | complex key |
| タブによるインデント | tab indentation |
| 浅すぎる子 | child indented less than its parent |
| 行 / 列 | line / column |
| スコープ | scope |
| パターン | pattern |
| コメント | comment |
| 読めない | cannot read |
| 正規表現なしでビルドされている | this binary was built without regular expressions |
| 大文字小文字を区別する / しない | case-sensitive / case-insensitive |

用語表に無い語は自分で決めてよいが、**報告に列挙する**（english-2 と突き合わせる）。

## 3. テスト

- 期待文字列を持つテスト（`tests/cli.rs`・各モジュールの `#[cfg(test)]`）を**新しい英語に合わせて更新**する。テストを消さない・緩めない
- `tests/readme.rs` は README の `console` ブロックを実行して照合する。**README の例に stderr の日本語が含まれていないことを確認**し、含まれていたら README 側は触らずに報告する（README は設計リナが直す）
- `make check` が緑

## 4. 完了条件

- `make check` 緑。**最後に必ず通す**
- `scopegrep --help` / `scopegrep --bogus` / `scopegrep x /nonexistent` / `scopegrep x scopegrep/testdata/unsupported-anchor.yml` / `scopegrep -e x .`（regex 無しビルド: `cargo run -p scopegrep -- -e x .`）の**実出力を報告に貼る**
- **コミットしない**。`git status --short` を報告
- 報告に含める: 変更ファイル一覧・テスト件数（変わらないはず）・`make check` の末尾・用語表に無かった語とその訳・迷った文言・満たせなかった点

## 5. 🔴 やってはいけないこと

- `fleet-top*/` を**触らない**（別の実装リナが同時に作業している。`Cargo.lock` にも触らない）
- 版を上げない（`Cargo.toml` の `version` はそのまま。版上げは設計リナ）
- `README*` / `CHANGELOG.md` / `docs/` を触らない
- 振る舞い・形式・終了コードを変えない。**文字列だけ**
- root `Cargo.toml` の lints・`clippy.toml`・`Makefile`・`xtask/` を変えない
- 文言を「良くする」ために増やさない。**同じ情報を英語で**。短くなるのはよい
