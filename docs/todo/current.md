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
| **PR #2 のレビューとマージ** | 🔲 **施主** |

---

## 次の1手: PR #2 を施主がレビューしてマージ

コミットは 4 段（core → CLI → README/規約 → カバレッジ）。設計リナが各段をレビュー済み、CI 緑。
🔴 マージ前に見てほしい判断（設計リナが下したもの・異論があれば戻す）:

- 列（`column`）は「値の先頭」ではなく **「一致の先頭」**（`rg --column` と同じ）
- ラベル（`name:`）は**クォートを1枚外す**（キーと同じ扱い。値は原文のまま）
- 人向け出力は `<file>:<line>: <path> = <value>`。README が約束していた `--show-scope` は**設けない**
- `scopegrep-core` の `Cargo.toml` に `readme = "../README.md"` を書いた
  （`clippy::cargo_common_metadata` が要求する。core 自身の README は持たない）
- カバレッジ下限は workspace 全体で行 90%（実測 92.21%）。`xtask` を除外していない

## マージ後の候補（どれも急がない・決め打ちしない）

- [ ] **部分集合を広げる**。候補: 複数ドキュメント（`---`・k8s manifest）、アンカー/エイリアス（compose）、
      複数行プレーンスカラー。🔴 **設計メモの表を先に更新し、エラーになるテストを通るテストに変えてから実装**
- [ ] コメント内ヒットを別枠で出す旗（例: `--comments`）。スキャナはコメント位置を知っているので依存なしで足せる
- [ ] TOML / JSON（同じ API で足す。設計メモの非目標を外す判断が先）
- [ ] `docs/benchmarks/` に `grep` との比較（QLT-009。**測ってから書く**）
- [ ] `cargo-deny`（依存を1つ足す ADR と同時に。今は依存 0 なので対象が無い）

## 規約側の残り（急がない）

- [ ] `CNF-005` 不変条件を持つ型の単独モジュール検査（型情報が要るか要検討）
- [ ] `macro_rules!` の判断（RS-010 の planned 残り。現在 `macro_rules!` は 0 件）
