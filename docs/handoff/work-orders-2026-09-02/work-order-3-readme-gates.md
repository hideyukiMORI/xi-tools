# 作業指示 3 — README を実装に合わせる・規約の planned を実装で active にする

担当: 実装リナ（Opus 5）/ 発注: 設計リナ（Fable 5.1）/ 2026-09-02

前提: 作業指示 1・2 が入っている（`scopegrep-core` と `scopegrep` バイナリが動き、`make check` 緑）。

## 0. 最初に読む

1. `/home/xi/docker/xi-tools/CLAUDE.md` — **地雷2（README の出力例）と地雷4（planned→active の条件）が今回の主題**
2. `/home/xi/docker/xi-tools/README.md`
3. `/home/xi/docker/xi-tools/docs/design/scopegrep.md`
4. `/home/xi/docker/xi-tools/docs/coding-rules.md` の ARC-003・RS-015・RS-010・第6節（CNF 一覧）・QLT-007
5. `/home/xi/docker/xi-tools/docs/quality/gate-proofs.md` — 証明の書き方
6. `/home/xi/docker/xi-tools/xtask/src/check.rs` と `main.rs` — CNF の足し方（検査関数＋テスト2種＋`main` での配線）

## 1. README を実装に合わせる（地雷2）

🔴 **README は成果物の本体**（CLAUDE.md）。「何が問題だったか」の物語は残し、**出力例だけを実際の出力に差し替える。盛らない。**

- `scopegrep` 節の `$ scopegrep 'cancelled()' .github/workflows/ --show-scope` の例を、
  **実際に `cargo run -p scopegrep -- 'cancelled()' scopegrep-core/testdata/` を実行した出力**に置き換える
  （コマンドは README の位置＝リポジトリ root で実行した形で書く。`--show-scope` は存在しないので消す）
- 直前の `grep -n` の例も、同じ fixture に対する**実際の `grep -n 'cancelled()' scopegrep-core/testdata/workflow-with-comment.yml` の出力**にする
  （今の README の `nene-corpus/...` の行は実在のリポの例であって fixture ではない。**fixture で5行出る（うち3行がコメント）ことが、コメントの偽陽性をそのまま見せる**）
- 物語（欠陥側と正しい用法側の説明）は fixture の2つのステップ（`Audit` / `Upload Playwright report`）に合わせて言い換える。事実関係（2026-09-01 の実害）は変えない
- 「設計上の判断」に以下を足す。**すべて実装済みの事実だけ**:
  - 対応する YAML は部分集合で、**外はエラーにする**（設計メモの表へリンク）。読めるもの／エラーにするものの要約
  - `--json`（JSON Lines・JSON Pointer）と終了コード 0/1/2
  - 依存 0・中核は `no_std`
- ツール一覧表の状態を `🔲 設計中` から実態に合わせる（例: `🟢 動く（YAML の部分集合）`）。**「完成」とは書かない**
- 設計メモの「D-2 実測」節が埋まっていれば、README から1行で参照する（埋まっていなければ触らない）

## 2. README の出力例を機械で守る（地雷2 を planned から active へ）

`scopegrep/tests/readme.rs` を足す:

- `../README.md` を読み、```` ```console ```` ブロックの中の `$ scopegrep …` 行を全部拾う
- 各行を引数に分解して（空白区切り・`'…'` のクォートを剥がす程度でよい）バイナリ（`env!("CARGO_BIN_EXE_scopegrep")`）を**リポジトリ root を cwd にして**実行し、
  標準出力が README の続く行（次の `$` か ```` ``` ```` まで）と**完全一致**することを検証する
- `$ grep -n …` の行も同様に `grep` を実行して比較する（`grep` が無い環境は考えなくてよい。CI は ubuntu）
- 少なくとも1つの `$ scopegrep` ブロックが見つかることを検証する（0件なら失敗＝README から例が消えたら気づく）

## 3. 規約の planned を active にする（実装の事実に基づいて・地雷4）

🔴 **順番を守る: 実装 → 意図的な違反で発火を確認 → `gate-proofs.md` に実出力 → `coding-rules.md` を active に。** 逆にしない。

### 3-a. CNF-007 — `-core` で終わるクレートは `#![no_std]` を宣言する（ARC-003 / RS-015）

- `xtask` に検査を足す: workspace の member のうち名前が `-core` で終わるものは、`src/lib.rs` の**桁 0 の最初の属性行群に `#![no_std]` がある**こと。無ければ `CNF-007` 違反
- テスト2種（発火する／正しいコードでは発火しない）を `check.rs` のテストモジュールに足す
- 発火の証明: `scopegrep-core/src/lib.rs` から一時的に `#![no_std]` を消して `make conformance` を走らせ、実出力を `gate-proofs.md` の CNF 表に足す。戻す
- 加えて **C-9** として「`scopegrep-core` で `use std::fs;` を書く」→ `cargo build -p scopegrep-core` の実出力（`E0433` のはず）を「言語側の証明」表に足す。戻す
- `coding-rules.md`: ARC-003 を **active**（`#![no_std]` の維持＝`CNF-007`、到達不能性はコンパイラ）に、RS-015 を **active**（同上）に。第6節の表に `CNF-007` 行を足す。**文言は事実だけ**

### 3-b. CNF-008 — `build.rs` を置かない（RS-010）

- `xtask`: workspace 配下（`target/` を除く）に `build.rs` が存在したら `CNF-008` 違反。各 `Cargo.toml` の `build = "..."` 指定も違反
- テスト2種・発火の証明（一時的に空の `scopegrep-core/build.rs` を置く → 実出力 → 消す）・`gate-proofs.md`・`coding-rules.md` の RS-010 を「`build.rs` の禁止＝`CNF-008` active」に（`macro_rules!` の判断は planned のまま）

### 3-c. `docs/coding-rules.md` の CNF 表・`.github/pull_request_template.md` の planned 一覧を同期する

PR テンプレートに planned 行の一覧があるなら、active にしたものを外す。

## 4. 完了条件

- `make check` 緑
- `git status --short` を報告。**コミットしない**
- 報告に含める: README の差し替え前後の差分の要点・`readme.rs` のテスト件数・CNF-007/008 の発火の実出力・`coding-rules.md` で変えた行（planned→active にしたものを全部列挙）・満たせなかった点

## 5. 🔴 やってはいけないこと

- README に**実行していない出力**・**測っていない数値**を書かない。「速い」「軽い」等の形容を足さない（QLT-009）
- planned を、実装と証明を伴わずに active と書かない
- root `Cargo.toml` の `[workspace.lints]`・`clippy.toml`・`Makefile` を変えない（`Makefile` は検査を足す場合だけ可。今回は要らないはず）
- 依存を足さない

## 6. 追記（2026-09-02・CLI レビュー後）

- fixture に対する実出力は次のとおり（列は「一致の先頭」で 18）。README の例はこれを**実行して**貼ること:
  ```
  scopegrep-core/testdata/workflow-with-comment.yml:33: jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .if = ${{ !cancelled() }}
  scopegrep-core/testdata/workflow-with-comment.yml:46: jobs.e2e.steps[2] "Upload Playwright report" .if = ${{ !cancelled() }}
  ```
- 小さな磨き: `ParseError` の表示で `行内で閉じないフロー記法 は読めない構文である` のように**助詞の前に空白が入る**。`UnsupportedSyntax` / `MalformedInput` の `Display` と組み立て側を見て、空白が入らない形（例: `行内で閉じないフロー記法は読めない`）に直す。CLI 統合テストの期待値も追随させる
- README の `grep -n` 例は fixture に対して **5 行**出る（4・29・30 がコメント、33・46 が値）。この 5 行をそのまま貼ってから「うち 3 行はコメント」と書く。実行して貼ること
