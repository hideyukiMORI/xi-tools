# 作業指示 8 — 0.1.0 を配れる形にする（英語 README・版・Release workflow・publish の準備）

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

施主の判断: **機能追加は止める。英語 README にする。配れる形にする。** それ以外（アンカー・複数ドキュメント・TOML/JSON）は保留。

ブランチ `release/0.1.0`（`main` から。切り替えない・コミットしない）。作業ディレクトリ `/home/xi/docker/xi-tools`。

## 0. 最初に読む

1. `/home/xi/docker/xi-tools/CLAUDE.md` — 第一目的・**README を盛らない**・地雷 2（README の例はテストが照合する）
2. `/home/xi/docker/xi-tools/README.md`（現行・日本語）
3. `/home/xi/docker/xi-tools/scopegrep/tests/readme.rs`
4. `/home/xi/docker/xi-tools/.github/workflows/ci.yml`・`Makefile`・両 `Cargo.toml`
5. `/home/xi/docker/xi-tools/docs/coding-rules.md` QLT-003（CI は `make check` を呼ぶだけ）・QLT-009（性能の主張は実測を伴う）

## 1. 英語 README（主）と日本語 README（副）

- **`README.md` を英語にする。** 現行の日本語は **`README.ja.md`** に移す（内容は現行のまま）。両ファイルの先頭に相互リンクを 1 行（`English | 日本語`）
- 英訳は**忠実に**。数字・事実・失敗の記述を 1 つも変えない・盛らない・形容を足さない。`console` ブロックは**両ファイルで同一**（コマンドと出力は言語に依らない）
- 日本語の fixture 内容（`# 1) 散文。…` などコメント行）が出力に出るのはそのまま（fixture は変えない）
- 文体: 簡潔な技術英語。見出しは README.ja.md と 1 対 1。翻訳で迷った用語は「所属 = scope」「部分集合 = subset」「一致の先頭 = start of the match」「配線点 = wiring point」で統一
- **`tests/readme.rs` は `README.md` と `README.ja.md` の両方を照合する**ようにする（同じ関数を 2 ファイルに回す。どちらかで `$ scopegrep` の例が 0 件なら落ちる）
- README に **Install** 節を足す（英語・日本語とも）:
  ```
  cargo install scopegrep                    # fixed-string search, zero dependencies
  cargo install scopegrep --features regex   # adds -e/--regex (3 crates: regex, regex-automata, regex-syntax)
  ```
  と、GitHub Releases に各 OS の binary（regex 入り）がある旨。🔴 **publish は PR マージ直後に施主が行う。** それまで README のこの 2 行は「まだ真でない」ので、PR 本文にその旨を書く（設計リナが書く）
- `scopegrep-core` の `Cargo.toml` の `readme` は `../README.md` のまま（英語になる）

## 2. 版を `0.1.0` に

- `scopegrep-core` と `scopegrep` の `version` を `0.1.0`。`scopegrep` の依存 `scopegrep-core = { path = "../scopegrep-core", version = "0.1.0" }`
- `Cargo.lock` を追随（`--locked` を通す）
- **`CHANGELOG.md`**（英語・Keep a Changelog 形式）に `0.1.0 — 2026-09-02` を書く。項目は今日の PR #1〜#4 の**事実だけ**（設計メモ・日報から拾う）。「速い」等の形容は書かない

## 3. Release workflow

`.github/workflows/release.yml`。**トリガーはタグ `v*` の push だけ。**

- job `verify`（ubuntu）: `ci.yml` の check job と同じ導入（`cargo-llvm-cov` / `cargo-deny` の install-action）→ `make check`。
  🔴 **タグと `Cargo.toml` の版が一致しない（`v0.1.0` ↔ `0.1.0`）なら失敗**させる（版の正本は `Cargo.toml`。タグを打ち間違えて別版の binary が出る事故を防ぐ）。この判定は `Makefile` に `check-version` ターゲットとして置き（`TAG=` を受ける）、workflow はそれを呼ぶだけ（QLT-003）
- job `build`（`needs: verify`・matrix: `ubuntu-latest` x86_64 / `macos-latest` arm64 / `windows-latest` x86_64）:
  `cargo build --release --locked -p scopegrep --features regex` → `scopegrep-<version>-<target>.tar.gz`（Windows は `.zip`）に binary・`LICENSE`・`README.md` を入れる。`sha256` も出す
- job `release`（`needs: build`）: `gh release create <tag> --generate-notes` に成果物を添付。**サードパーティの release action は使わず `gh` CLI**（runner に入っている）。`permissions: contents: write` はこの job だけ
- 版は `rust-toolchain.toml` が決める。workflow に `toolchain:` を書かない（地雷 3）
- 🔴 実際にタグは打たない（施主がやる）。workflow の YAML は **`scopegrep` 自身で読めること**を確認する（`scopegrep --scope '/jobs/*/steps/*/run' '' .github/workflows/release.yml` が終了 0 か 1）。これは自分の道具で自分の設定を検査する 1 行で、README に書いてよい（**実行して**）

## 4. publish の準備

- `Makefile` に `package` ターゲット: `cargo package -p scopegrep-core --locked` と `cargo package -p scopegrep --locked --no-verify`
  （後者の `--no-verify` は core が crates.io に無い段階では検証ビルドが不可能なため。理由をコメントに）。`make check` には**入れない**（ネットワークと時間がかかる）
- `docs/release.md`（日本語・内部手順）: 版上げ → CHANGELOG → `make check` → `make package` → PR → マージ → `cargo login` → `cargo publish -p scopegrep-core` → 数分待つ → `cargo publish -p scopegrep` → `git tag v0.1.0 && git push origin v0.1.0` → Release workflow が binary を上げる。**順番を変えると失敗する箇所**（core より先に bin を publish できない）を明記
- `cargo package` を実際に走らせ、両方の `.crate` が作れることと**中に `target/`・`testdata` 以外の不要物が入っていない**ことを `--list` で確認して報告に貼る（`scopegrep-core/testdata` はテストが使うので入ってよい）

## 5. テスト・完了条件

- `make check` 緑（両 README の照合を含む）
- `make package` が通る
- `make check-version TAG=v0.1.0` が通り、`TAG=v0.2.0` が落ちる（実出力を報告に）
- **コミットしない**。`git status --short` を報告
- 報告: 変更ファイル・テスト件数・`make check` 末尾・`cargo package --list` の要約・英訳で判断に迷った箇所（原文と訳を並べて）・満たせなかった点

## 6. 🔴 やってはいけないこと

- 英訳で事実・数字を変えない。英語版にだけある文を作らない（両言語は 1 対 1）
- `cargo publish` を打たない（dry-run も、core が無い段階で bin 側は失敗するので不要）。タグを打たない
- ゲート設定を緩めない。CI 側にだけ検査を書かない（版の一致判定も `Makefile`）
- 機能を足さない・変えない。fixture を変えない

## 7. 追記 — crates.io 向けのメタデータ

- 両 `Cargo.toml` の `description` を**英語**にする（crates.io の一覧に出る 1 行）。
  例: scopegrep → `grep that tells you where in the structure a hit belongs (YAML)`、
  scopegrep-core → `no_std core of scopegrep: parses a YAML subset and returns the scope of each hit`。事実だけ・形容なし
- `keywords`（5 個まで）と `categories` は crates.io の既存カテゴリ名であること（`cargo package` が警告しないこと）
