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
| PR → CI → マージ | 🔲 |

### 施主に確認すること（`fleet-top` を出すかどうか）

- **版と publish**: 両 crate は `0.0.0` のまま。crates.io に出すなら `0.1.0` に上げ、`docs/release.md` の手順（core → bin）。
  Release workflow（`release.yml`）は `scopegrep` の binary しか作らないので、`fleet-top` も配るなら matrix に足す
- **README の例は照合していない**（出力が GitHub とローカルの状態に依存する）。この扱いでよいか
- `--no-github` でローカルが読めなかった行は GitHub 3 列が `n/a`（聞いていない）。旗なしなら `?`。この非対称は意図どおり

🔴 README に書く数字は `docs/benchmarks/fleet-top.md` からだけ取る（QLT-009）。

## `scopegrep` の保留（困った人が現れてから）

- アンカー・複数ドキュメント・継続行（手元の corpus に 0 件）
- TOML / JSON（同じ API で足す。設計メモの非目標を外す判断が先）
- `docs/benchmarks/` に `grep` との比較（測ってから）

## 規約側の残り（急がない）

- `CNF-005` 不変条件を持つ型の単独モジュール検査（型情報が要るか要検討）
- `macro_rules!` の判断（RS-010 の planned 残り。現在 0 件）
