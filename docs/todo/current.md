# 今やること — xi-tools

> 2026-09-02 更新（第2セッション）。横断 TODO（`_work/board.txt`）の対象外。**このリポの TODO はここで持つ**（CLAUDE.md）。

## 状態

| | |
| --- | --- |
| workspace の足場・CI | ✅ 2026-09-01 |
| コーディング規約と機械強制 | ✅ 2026-09-02（PR #1） |
| D-1〜D-4 | ✅ 2026-09-02 確定（`docs/design/scopegrep.md`。D-2 は 6 crate を実測して決定） |
| `scopegrep-core`（no_std スキャナ＋検索） | ✅ 2026-09-02（PR #2） |
| `scopegrep` CLI（人向け / `--json` / 終了コード） | ✅ 2026-09-02（PR #2） |
| README を実出力に合わせ、テストで照合 | ✅ 2026-09-02（PR #2・`tests/readme.rs`） |
| ARC-003 / RS-015 / RS-010(build.rs) / QLT-008 を active に | ✅ 2026-09-02（PR #2・CNF-007 / CNF-008 / `make coverage`） |
| PR #2 のレビューとマージ | ✅ 2026-09-02 マージ済み |
| `--comments`（コメント内ヒットを別枠で返す） | ✅ 2026-09-02（`feat/scopegrep-comments`） |

---

## 0.1.0 は出た（2026-09-02）

| | |
| --- | --- |
| crates.io | [`scopegrep`](https://crates.io/crates/scopegrep) / [`scopegrep-core`](https://crates.io/crates/scopegrep-core) 0.1.0 |
| GitHub Release | [v0.1.0](https://github.com/hideyukiMORI/xi-tools/releases/tag/v0.1.0) — Linux x86_64 / macOS arm64 / Windows x86_64（regex 入り・sha256 付き） |
| 手元 | `cargo install scopegrep --features regex` で crates.io 版に入れ替え済み |

`scopegrep` の機能追加は施主判断で停止中。困った人（施主自身を含む）が現れたら、それを実測にして再開する。

## 進行中: `fleet-top`（施主指示「開発開始」2026-09-02）

正本は [`docs/design/fleet-top.md`](../design/fleet-top.md)（実測・F-1〜F-5 確定）と [ADR 0003](../adr/0003-fleet-top-fetches-github-via-chunked-graphql.md)。

| 手 | 状態 |
| --- | --- |
| 試作で測る（REST 並列・GraphQL 1 本・GraphQL 分割並列・ローカル） | ✅ 2026-09-02。60 リポ 1.4 s で境界を越えた |
| 設計メモに実測と決定を書く・ADR 0003 | ✅ 2026-09-02 |
| 作業指示 1: `fleet-top-core` 前半（JSON パーサ・`Day`・remote URL） | ✅ 2026-09-02（テスト 59・`#[expect]` 0） |
| 作業指示 2: `fleet-top-core` 後半（porcelain v2・GraphQL クエリと応答・表の整形） | ✅ 2026-09-02（テスト 153・設計メモの例の矛盾 2 点を実装リナが発見） |
| 作業指示 3: `fleet-top` bin（引数・走査・並列サブプロセス・出力・終了コード） | ✅ 2026-09-02（テスト 46・実機 60 リポ 1.6〜1.8 s。理由の無い `?` をレビューで修正） |
| README（両言語）・`docs/benchmarks/fleet-top.md`・CHANGELOG・ARC-001 の記述更新 | ✅ 2026-09-02（設計リナが直接書いた。作業指示 4 は出していない） |
| PR #8 → CI → マージ | ✅ 2026-09-02 |
| 出す判断 | ✅ 施主「出していい」（2026-09-02）。第三者が使える前提（`gh` ログイン済み・直下に並ぶ・GitHub のみ）と、文言が日本語であることを説明した上で |
| 版 0.1.0・タグ方式 `<tool>-vX.Y.Z`・Release workflow の一般化・手順書 | ✅ PR #9（2026-09-02） |
| publish（core → bin）→ タグ `fleet-top-v0.1.0` → Release workflow | ✅ 2026-09-02。[crates.io `fleet-top`](https://crates.io/crates/fleet-top) / [`fleet-top-core`](https://crates.io/crates/fleet-top-core)・[Release](https://github.com/hideyukiMORI/xi-tools/releases/tag/fleet-top-v0.1.0) 3 OS。手元は crates.io 版に入れ替え済み |

**`fleet-top` も出た。次の 1 手は無い**（`scopegrep` と同じく、困った人が現れてから）。

## 仕上げ: 利用者向け文言の英語化（施主指示「仕上げる方やって」2026-09-02）

| | 状態 |
| --- | --- |
| 作業指示 english-1（scopegrep）・english-2（fleet-top）を並行で | ✅ 2026-09-02。文字列だけ・テスト数不変・`make check` 緑 |
| 両ツールの用語の突き合わせ（`line N:`・`given more than once …`・`no … follows`・見出しの `—`） | ✅ 設計リナが 2 か所を揃えた |
| 版 0.1.1（4 crate）・CHANGELOG | ✅ |
| publish（4 crate）→ タグ `scopegrep-v0.1.1` / `fleet-top-v0.1.1` | ✅ 2026-09-02（施主確認の上）。Release workflow 2 本とも 3 OS 成功。手元は両方 0.1.1 |

**2 本とも 0.1.1 まで出た。次の 1 手は無い**（困った人が現れてから）。

### 後で判断すること

- `--no-github` でローカルが読めなかった行は GitHub 3 列が `n/a`（聞いていない）。旗なしなら `?`。この非対称は意図どおり

🔴 README に書く数字は `docs/benchmarks/fleet-top.md` からだけ取る（QLT-009）。

## `scopegrep` の保留（困った人が現れてから）

- アンカー・複数ドキュメント・継続行（手元の corpus に 0 件）
- TOML / JSON（同じ API で足す。設計メモの非目標を外す判断が先）
- `docs/benchmarks/` に `grep` との比較（測ってから）

## 規約側の残り（急がない）

- `CNF-005` 不変条件を持つ型の単独モジュール検査（型情報が要るか要検討）
- `macro_rules!` の判断（RS-010 の planned 残り。現在 0 件）
