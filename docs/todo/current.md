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

## 次の1手: `fleet-top` の試作で測る（施主決定 2026-09-02）

正本は [`docs/design/fleet-top.md`](../design/fleet-top.md)。候補の全体は [`docs/design/candidates.md`](../design/candidates.md)。

1. **試作（1 時間・スクラッチパッド・リポに入れない）**: `gh api` を 8 / 16 / 32 並列で 126 本叩いて壁時計時間を測る。ローカル側も同様。レート制限に当たるかも見る
2. 実測を設計メモに書き、**3 秒を切らなければ設計を変える**
3. F-1〜F-5 を決める。**F-3（`tokio` 等）は依存の ADR**（前例 ADR 0002）
4. 作業指示を書いて実装リナへ（型は `docs/handoff/work-orders-2026-09-02/`）

🔴 「並列化で 3 秒」は想定。**測るまで README に書かない**（QLT-009）。

## `scopegrep` の保留（困った人が現れてから）

- アンカー・複数ドキュメント・継続行（手元の corpus に 0 件）
- TOML / JSON（同じ API で足す。設計メモの非目標を外す判断が先）
- `docs/benchmarks/` に `grep` との比較（測ってから）

## 規約側の残り（急がない）

- `CNF-005` 不変条件を持つ型の単独モジュール検査（型情報が要るか要検討）
- `macro_rules!` の判断（RS-010 の planned 残り。現在 0 件）
