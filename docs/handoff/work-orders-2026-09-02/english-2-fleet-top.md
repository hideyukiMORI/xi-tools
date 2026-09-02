# 作業指示 english-2 — `fleet-top` / `fleet-top-core` の利用者向け文言を英語にする

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

## 0. 最初に読む（この順・飛ばさない）

1. `/home/xi/docker/xi-tools/CLAUDE.md` — 第一目的（**public に示す**）と地雷 6 件
2. `/home/xi/docker/xi-tools/docs/coding-rules.md` — 規約。今回は文字列しか触らないが、`make check` は全部通す
3. `/home/xi/docker/xi-tools/fleet-top/src/output.rs`・`usage_error.rs`・`cli.rs`・`run.rs`・`local.rs`・`github.rs`・`reason.rs` — bin 側の文言
4. `/home/xi/docker/xi-tools/fleet-top-core/src/json_error.rs`・`json_error_kind.rs`・`json_parser.rs`・`remote_error.rs`・`porcelain_error.rs`・`porcelain_error_kind.rs`・`porcelain_line.rs`・`table.rs` — core 側の `Display` 文言
5. `/home/xi/docker/xi-tools/fleet-top/tests/cli.rs` — 期待文字列を持つ統合テスト（`fleet-top: alpha: 枝が 100 本を超えている` 等）

## 1. 何を、なぜ

README は英語なのに、実行すると `--help` も stderr も日本語が出る。第三者が最初に踏む段差なので、
**利用者の目に触れる文字列**を英語にする。`scopegrep` 側は別の作業指示（english-1）で同時に進めるので、
**両ツールで同じ語を同じ英語にする**（下の用語表）。

### 対象（変える）

- `--help` の全文（見出し `usage:` / `arguments:` / `options:` / `columns:` / `exit status:` の**構造は変えない**。中身を英語に）
- 使い方エラー（`UsageError` の `Display`）と `output::usage` が出す文
- stderr に出る全ての文: `fleet-top: <name>: <理由>` の理由部分（`git status が失敗した（理由の出力が無い）`・`gh の出力が UTF-8 ではない`・
  `枝が 100 本を超えている。STALE は数えていない` 等）、`output::unreadable` / `output::clock` の文
- `fleet-top-core` の公開エラー型の `Display`（`JsonError` / `JsonErrorKind` / `RemoteError` / `PorcelainError` / `PorcelainErrorKind`）
- 要約行 `fleet-top: 60 repos, 45 on GitHub, 1.6s` は**既に英語**。変えない

### 対象外（変えない）

- コード内のコメント・doc コメント・テスト名・`expect("…")` / `expect_err("…")` / `assert!` のメッセージ
- 表の見出し・記号（`REPO` … `STALE`・`-` / `?` / `n/a` / `(none)` / `(detached)` / `ok` / `FAIL` / `...`）。**既に英語**
- 出力の形式・終了コード。**文字列の英語化だけ**で振る舞いを変えない
- `Cargo.toml` の `description`

## 2. 英語の書き方（両ツール共通）

- 1 行メッセージは**文頭小文字・末尾ピリオド無し**（`grep` / `git` / `cargo` の流儀。例: `fleet-top: alpha: gh not found`）
- `--help` の説明文も文頭小文字・ピリオド無し。複数文になるときだけピリオドで区切る
- ASCII の句読点だけ（全角記号・`——`・`「」` を使わない。引用は `` ` `` か `'`）
- 値は原文のまま埋め込む（`{name}` / `{path}` / `{message}` の位置を変えない）
- 「〜ではない」は `is not …`、「〜が無い」は `missing …` / `no …`、「読めない」は `cannot read …`、「想定外の形」は `unexpected …`

| 日本語 | 英語 |
| --- | --- |
| 使い方 | usage |
| 枝 | branch |
| 既定枝 | default branch |
| 上流 | upstream |
| 未追跡 | untracked |
| 変更・未追跡・衝突のエントリ数 | changed, untracked and conflicted entries |
| 古い枝 | stale branches |
| 日数 | days |
| 直下 | direct children |
| 再帰しない | not recursive |
| 取れなかった | could not be determined |
| 理由は標準エラーに 1 行ずつ | one reason per line on stderr |
| 読めない | cannot read |
| 時計が読めない | cannot read the system clock |
| GitHub にそのリポジトリが無い | repository not found on GitHub |
| GitHub が拒んだ | GitHub rejected the request |
| 応答の形が想定と違う | unexpected response shape |
| 枝が 100 本を超えている。STALE は数えていない | more than 100 branches; STALE was not counted |
| `# branch.head` が無い | missing `# branch.head` |
| 入れ子が深すぎる | nesting too deep |
| 予期しない文字 | unexpected character |
| 入力が値の途中で終わっている | unexpected end of input |
| 値の後に余分な文字がある | trailing characters after the value |
| 孤立サロゲート | lone surrogate |
| 制御文字 | control character |
| 数の書き方が JSON の文法に合わない | invalid number |
| N 文字目（0 起点） | at character N（0 起点はそのまま。例: `at character 12: …`） |

用語表に無い語は自分で決めてよいが、**報告に列挙する**（english-1 と突き合わせる）。

## 3. テスト

- 期待文字列を持つテスト（`tests/cli.rs`・各モジュールの `#[cfg(test)]`。`json_error.rs` の `Display` テスト等）を**新しい英語に合わせて更新**する。テストを消さない・緩めない
- `make check` が緑

## 4. 完了条件

- `make check` 緑。**最後に必ず通す**
- `fleet-top --help` / `fleet-top --bogus` / `fleet-top /nonexistent` / `fleet-top --stale-days x` の**実出力を報告に貼る**。
  加えて `cargo run --release -p fleet-top -- /home/xi/docker 2>&1 >/dev/null` の stderr（理由の行と要約行）を貼る
- **コミットしない**。`git status --short` を報告
- 報告に含める: 変更ファイル一覧・テスト件数（変わらないはず）・`make check` の末尾・用語表に無かった語とその訳・迷った文言・満たせなかった点

## 5. 🔴 やってはいけないこと

- `scopegrep*/` を**触らない**（別の実装リナが同時に作業している。`Cargo.lock` にも触らない）
- 版を上げない（`Cargo.toml` の `version` はそのまま。版上げは設計リナ）
- `README*` / `CHANGELOG.md` / `docs/` を触らない（README の例の stderr 行は設計リナが実出力で差し替える）
- 振る舞い・形式・終了コードを変えない。**文字列だけ**
- root `Cargo.toml` の lints・`clippy.toml`・`Makefile`・`xtask/` を変えない
- 文言を「良くする」ために増やさない。**同じ情報を英語で**。短くなるのはよい
