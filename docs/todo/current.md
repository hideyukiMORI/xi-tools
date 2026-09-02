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

## 次の1手: PR #4 を施主がレビューしてマージ

コミットは 2 段（`--scope` / `-i` / パス省略 / 除外 → feature `regex` ＋ `cargo-deny`）。設計リナが各段をレビュー済み。
🔴 マージ前に見てほしい判断:

- `-e` があるときは位置引数を全部パスとして読む（needle の位置は無くなる。排他の検出は機械的に不能だった）
- `make deny` は `--deny license-not-encountered` 付き。使っていないライセンスを `allow` に書くと落ちる
- `deny.toml` は `[graph] all-features = true`。opt-in の依存も検査グラフに入れる
- `Query` の `Matcher` は同一性でのみ等しい（中身を比較できないため）
- PR テンプレートに ARC-004 の人手チェック 1 行を足した（不要なら消す）

マージ後: `cargo install --path scopegrep --locked --features regex` で入れ直す（設計リナがやる）。

## PR #4 マージ後の候補（どれも急がない・決め打ちしない）

- [ ] **部分集合を広げる**。候補: 複数ドキュメント（`---`・k8s manifest）、アンカー/エイリアス（compose）、
      複数行プレーンスカラー。🔴 **設計メモの表を先に更新し、エラーになるテストを通るテストに変えてから実装**
- [ ] TOML / JSON（同じ API で足す。設計メモの非目標を外す判断が先）
- [ ] `docs/benchmarks/` に `grep` との比較（QLT-009。**測ってから書く**）
- [x] `cargo-deny`（✅ 2026-09-02・ADR 0002 と同じ PR で導入。`deny.toml` / `make deny` / 証明 P-18〜P-20）

## 規約側の残り（急がない）

- [ ] `CNF-005` 不変条件を持つ型の単独モジュール検査（型情報が要るか要検討）
- [ ] `macro_rules!` の判断（RS-010 の planned 残り。現在 `macro_rules!` は 0 件）
