# 作業指示 fleet-top-3 — `fleet-top`（bin: 引数・走査・並列サブプロセス・出力・終了コード）

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

## 0. 最初に読む（この順・飛ばさない）

1. `/home/xi/docker/xi-tools/CLAUDE.md` — 第一目的と地雷 6 件
2. `/home/xi/docker/xi-tools/docs/design/fleet-top.md` — **今回の仕様の正本**。「アーキテクチャ」「CLI」「出力の形」「決定 F-1〜F-5」
3. `/home/xi/docker/xi-tools/docs/adr/0003-fleet-top-fetches-github-via-chunked-graphql.md`
4. `/home/xi/docker/xi-tools/fleet-top-core/src/lib.rs` と各モジュールの公開 API（前 2 回で作ったもの。**変えない**）
5. `/home/xi/docker/xi-tools/scopegrep/src/` — **この規約下で書かれた bin の実例**。`main.rs`（配線点）・`cli.rs`（手書きの引数解析）・`argument.rs`・`invocation.rs`・`options.rs`・`outcome.rs`（終了コード）・`output.rs`（出力の唯一の場所・`#[expect(print_*)]` の形）・`usage_error.rs`・`run.rs` に**倣う**。`scopegrep/tests/cli.rs` の書き方も
6. `/home/xi/docker/xi-tools/scopegrep/Cargo.toml`
7. `/home/xi/docker/xi-tools/docs/coding-rules.md` — 特に RS-005（`exit` forbid・終了コードは `main` から返す）・RS-014・RS-015・RS-016

## 1. 作るもの

workspace に **`fleet-top`** クレート（bin）を足す。`/home/xi/docker/xi-tools/fleet-top/`。

- `Cargo.toml` は `scopegrep/Cargo.toml` に倣う。`name = "fleet-top"`・`version = "0.0.0"`・
  `description = "one screen for the state of many git repositories: branch, dirty, ahead/behind, open PRs, CI, stale branches"`・
  `readme = "../README.md"`・`keywords` / `categories` は妥当に・`[lints] workspace = true`・
  `[dependencies] fleet-top-core = { path = "../fleet-top-core", version = "0.0.0" }`（`path` と `version` の両方。理由は scopegrep の Cargo.toml のコメント）。
  `exclude` / `[features]` は書かない
- root `Cargo.toml` の `members` に足す（`"fleet-top-core"` の前に `"fleet-top"`。`scopegrep` の並びに揃える）。`Cargo.lock` を cargo に更新させる
- **依存は `fleet-top-core` だけ。** 引数解析も並列も手書き（ADR 0003）

### CLI

```
fleet-top [DIR] [--stale-days N] [--no-github]
fleet-top --help
fleet-top --version
```

- `DIR` は 1 つまで（既定 `.`）。2 つ以上・知らない旗・`--stale-days` の値が無い／`u32` に読めない → 使い方エラー（終了 2・stderr に 1 行＋使い方）
- `--stale-days N` 既定 30。`--` より後は旗として解釈しない（scopegrep と同じ）
- `--help` は使い方を stdout に。`--version` は `fleet-top <CARGO_PKG_VERSION>` を stdout に。どちらも終了 0
- `DIR` が読めない（存在しない・ディレクトリでない・権限）→ 終了 2・stderr `fleet-top: <DIR>: <io error>`

### 走査（F-4）

- `DIR` **直下**のエントリのうち、ディレクトリで（シンボリックリンク先がディレクトリでもよい）、その中に `.git`（ディレクトリ**または**ファイル。worktree は `.git` がファイル）を持つものだけ。再帰しない
- 名前はファイル名（`OsStr` → `to_string_lossy`）。**ファイル名のバイト順**に並べる（`render` も並べ替えるが、走査の時点で決定的にしておく）
- 該当が 0 なら見出しだけ出して終了 0

### ローカル（リポごと・並列）

各リポについて、サブプロセスを 2 つ:

1. `git -C <path> status --porcelain=v2 --branch` → stdout を `fleet_top_core::parse_porcelain` に渡す
2. `git -C <path> remote get-url origin` → 終了 0 なら stdout（trim）を `parse_remote_url`。`Some(slug)` なら GitHub 対象。終了 0 以外・`None` なら `RemoteReport::NotOnGithub`

- 環境変数 `GIT_OPTIONAL_LOCKS=0` を付けて起動する（並列で `status` を打つと index の更新でロックを取り合う。読むだけなので要らない）
- `git` が起動できない・終了 0 以外・出力が UTF-8 でない・`parse_porcelain` が失敗 → `LocalReport::Unavailable`。理由を **stderr に 1 行** `fleet-top: <name>: <理由>`（理由は `io::Error` の表示・stderr の 1 行目・`PorcelainError` の表示のいずれか）。
  このとき remote の取得は試みず `RemoteReport::Unavailable`（`?`）にする

### GitHub（3 リポ × 1 リクエスト・並列。`--no-github` なら飛ばして全部 `NotOnGithub`）

- GitHub 対象のリポを走査順のまま `fleet_top_core::REPOS_PER_QUERY` 個ずつに切り、各塊で `gh api graphql -f query=<build_query(slugs)>` を起動する。
  `-f` の値はコマンドライン引数として渡す（シェルを介さない。`Command::arg`）
- **終了コードで捨てない。** stdout を `parse_json` → `parse_response(json, slugs)`。stdout が JSON として読めない（空・`gh` の使い方エラー等）→ その塊の全リポを `RemoteReport::Unavailable`。
  `gh` が起動できない（`ErrorKind::NotFound` 等）→ 同じく `Unavailable`
- `Err(RemoteError)` のリポは `Unavailable`。`NotFound` も `Unavailable`（`?`。「GitHub に無い」ではなく「読めない」——origin は GitHub を指しているのだから）
- stderr への理由: 塊の全リポが**同じ理由**で失敗した（起動失敗・JSON でない・全体 `Rejected`）ときは 1 行にまとめる:
  `fleet-top: alpha, beta, gamma: <理由>`。リポ単位の失敗は `fleet-top: <name>: <理由>`

### 並列（F-3）

- `std::thread::scope` ＋ `Mutex<VecDeque<...>>` のワーカープール。**ワーカー数は `min(32, タスク数)`**。結果は**タスクの投入順**で返す（`Vec<Option<T>>` を index で埋める等。反復順が実行順に依存しない・RS-016）
- 局面は 2 つ（ローカル → GitHub）。それぞれ全タスクを投げて待つ。ローカルの結果（slug）が GitHub の入力になる
- 1 つのモジュール（例 `parallel.rs`）に閉じる。`unsafe` なし・依存なし

### 出力（RS-014: `output.rs` だけが書く）

- stdout: `fleet_top_core::render(&rows, &freshness)` の結果をそのまま
- stderr: 理由の行（上記）→ 最後に要約 `fleet-top: <N> repos, <M> on GitHub, <T>s`（`T` は小数 1 桁。`--no-github` でも同じ形で `M` は「origin が GitHub だった数」ではなく **0**——問い合わせていないので）
- 「今日」は `SystemTime::now()` → `UNIX_EPOCH` からの秒 → `Day::from_unix_seconds`。**取得は `main`（配線点）で行い、値として渡す**（RS-015）。取れない（時計が 1970 より前）→ 終了 2・stderr 1 行
- 所要時間は `Instant` で `main` が測って `output` に渡す

### 終了コード（`outcome.rs`。`std::process::exit` は forbid・`main` から `ExitCode` を返す）

| | 意味 |
| --- | --- |
| 0 | 全行 `is_complete` |
| 1 | `?` を含む行がある |
| 2 | 使い方の誤り・`DIR` が読めない・時計が読めない |

## 2. テスト（QLT-007。これが無いと受け取らない）

### 単体（各モジュール末尾の `#[cfg(test)]`）

- `cli::parse`: 全形（既定 DIR・DIR 指定・`--stale-days 7`・`--no-github`・`--`・`--help`・`--version`・DIR 2 つ・知らない旗・`--stale-days` 値なし／非数）
- `parallel`: 結果が投入順であること（各タスクが投入順と**逆**の時間だけ `sleep` しても順が保たれる）。ワーカー数 1 と 32。タスク 0 個
- 塊への分割: 7 個 → 3+3+1。0 個 → 0 塊

### 統合（`fleet-top/tests/cli.rs`。`scopegrep/tests/cli.rs` の形）

一時ディレクトリに `git init` で架空のリポを作って走らせる（`Command` で `git` を叩く。`user.email` / `user.name` は `-c` で渡す）:

1. **`--no-github` の完全一致**: 3 つのディレクトリ——`alpha`（コミット 1 つ・clean・upstream 無し）、`beta`（コミット 1 つ・未追跡 1・変更 1・detached）、`not-a-repo`（`.git` 無し）——で
   ```
   REPO   BRANCH      DIRTY  AHEAD/BEHIND  PR   CI   STALE
   alpha  main        -      (none)        n/a  n/a  n/a
   beta   (detached)  2      (none)        n/a  n/a  n/a
   ```
   と stdout が完全一致し、終了 0、stderr の最後の行が `fleet-top: 2 repos, 0 on GitHub, ` で始まる。
   `alpha` の枝名は `git init -b main` で固定する（既定の枝名が環境で変わる）
2. **偽の `gh` で GitHub 経路を通す**（`#[cfg(unix)]`）: 一時ディレクトリに実行可能なシェルスクリプト `gh` を置き、`PATH` をそのディレクトリ**だけ**（＋ `git` のあるディレクトリ）にして起動する。
   スクリプトは受け取った引数を無視して固定の JSON を stdout に出す。`alpha` に `git remote add origin https://github.com/example-org/alpha.git` を足しておく。
   - 成功の JSON（`r0` が `main` / `SUCCESS` / PR 2 / 枝: `main`＝今日、`old`＝100 日前）→ `alpha` の行が `main  -  (none)  2  ok  1`、終了 0
     （`--stale-days 30` 既定。**日付は今日から計算して JSON に埋める**。固定の日付を書くと明日落ちる）
   - `{"message":"Bad credentials"}` だけを返す → `alpha` の GitHub 3 列が `?`、終了 1、stderr に `fleet-top: alpha: Bad credentials`
   - スクリプトが**無い**（`PATH` に `gh` が無い）→ 同じく `?`・終了 1・stderr に `alpha` と起動失敗の理由
   - `gh` が終了 1 で `{"data":{"r0":null},"errors":[{"type":"NOT_FOUND","path":["r0"],"message":"..."}]}` → `?`・終了 1
3. 存在しない DIR → 終了 2・stderr に DIR 名。`--stale-days x` → 終了 2。`--help` → 終了 0・stdout に `fleet-top [DIR]`。`--version` → `fleet-top 0.0.0`
4. 空のディレクトリ → 見出しだけ・終了 0

⚠️ テストは `CARGO_BIN_EXE_fleet-top` で binary を指す（scopegrep の cli.rs と同じ）。並列に走るテスト同士が同じ一時ディレクトリを使わないよう、テストごとに `std::env::temp_dir()` の下にユニークな名前（テスト名＋プロセス ID）で作り、最後に消す。

## 3. 完了条件

- `make check` が緑。**最後に必ず `make check` を通す**（カバレッジは workspace 全体で 90%。bin の主要経路は統合テストが通す）
- **実機 smoke**: `cargo run --release -p fleet-top -- /home/xi/docker` を打ち、stdout の先頭 5 行と stderr の要約行を報告に貼る（**リポ名はプロフィールで公開済みなので貼ってよい。枝名・PR タイトルは出力に無い**）。
  所要時間が 3 秒を超えたら、その旨を報告する
- `cargo test -p fleet-top` のテスト件数を報告する
- **コミットはしない**。`git status --short` で変更一覧を報告する
- 報告に含める: 変更ファイル一覧・テスト件数・`make check` の末尾出力・`#[expect]` の全列挙（`output.rs` の `print_stdout` / `print_stderr` は想定内）・規約で詰まった箇所とどう解いたか・仕様で曖昧だった点と自分の解釈・満たせなかった点

## 4. 🔴 やってはいけないこと

- `Cargo.toml`（root）の `[workspace.lints]`・`clippy.toml`・`Makefile`・`deny.toml`・`docs/coding-rules.md`・`xtask/` を**変更しない**
- `fleet-top-core` の**公開 API を変えない**。足りないものがあれば報告して止める（core にテストを足すのは可）
- `scopegrep` / `scopegrep-core` を触らない。`README*` / `CHANGELOG.md` / `docs/` を触らない（次の作業指示）
- `std::process::exit` を書かない（forbid）。`unwrap` / `expect` / 添字を書かない（テスト以外）
- `HashMap` を使わない。並列の結果を到着順で並べない
- **`git fetch` を打たない**。見るだけ（設計メモ「非目標」）
- `/home/xi/docker/xi-tools` 以外のリポジトリを変更しない。実機 smoke は読むだけ
- 時間がかかっても仕様を勝手に狭めない。狭めるなら報告に明記する

## 5. ヒント

- `output.rs` の関数ごとに `#[expect(clippy::print_stdout, reason = "RS-014: 出力は1箇所に集約する")]`（`scopegrep/src/output.rs` の形）
- `Command::output()` は stdout / stderr を両方取る。`gh` の終了コードは**見ない**（stdout が JSON なら読む）
- `std::thread::scope` の中で `Mutex<VecDeque<(usize, Task)>>` を取り、各ワーカーが `pop_front` → 実行 → `results.lock().insert(index, value)`。`results` は `Mutex<Vec<Option<T>>>` を長さ分 `None` で作っておく（`Default` は使えないので `(0..n).map(|_| None).collect()`）
- `Instant::elapsed().as_secs_f64()` を `{:.1}` で。**`f64` の比較・変換で `as` を書かない**
- `Day` は `Copy` なので値で渡す
- 統合テストの偽 `gh`: `#!/bin/sh\ncat <<'EOF'\n...json...\nEOF\n` を書いて `std::os::unix::fs::PermissionsExt` で `0o755` を付ける。`PATH` は `Command::env("PATH", ...)`。`git` の場所は `which git` 相当を自前で（`/usr/bin:/bin` を足せば足りる）
- 認知的複雑度 10・60 行: 「走査」「ローカル 1 リポ」「GitHub 1 塊」「行の組み立て」を別関数にすれば収まる
