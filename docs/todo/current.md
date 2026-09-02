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

**次の1手は無い。** 機能追加は施主判断で停止中。困った人（施主自身を含む）が現れたら、それを実測にして再開する。

## 0.1.0 の後の候補（**保留**・施主判断 2026-09-02。困った人が現れてから）

- [ ] **部分集合を広げる**。候補: 複数ドキュメント（`---`・k8s manifest）、アンカー/エイリアス（compose）、
      複数行プレーンスカラー。🔴 **設計メモの表を先に更新し、エラーになるテストを通るテストに変えてから実装**
- [ ] TOML / JSON（同じ API で足す。設計メモの非目標を外す判断が先）
- [ ] `docs/benchmarks/` に `grep` との比較（QLT-009。**測ってから書く**）
- [x] `cargo-deny`（✅ 2026-09-02・ADR 0002 と同じ PR で導入。`deny.toml` / `make deny` / 証明 P-18〜P-20）

## 規約側の残り（急がない）

- [ ] `CNF-005` 不変条件を持つ型の単独モジュール検査（型情報が要るか要検討）
- [ ] `macro_rules!` の判断（RS-010 の planned 残り。現在 `macro_rules!` は 0 件）
