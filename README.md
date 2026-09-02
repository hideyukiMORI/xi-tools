# xi-tools

自分の開発環境のために書いた Rust のツール群。

**汎用ツールとして設計していないものを含みます。** 数十のリポジトリを横断して作業する
個人の環境から出てきた道具なので、前提がその環境に寄っているものがあります。
それでも公開しているのは、**問題の立て方ごと残しておくため**です。

| ツール | 何を解くか | 状態 |
| --- | --- | --- |
| [`scopegrep`](./scopegrep) | grep が返さない「**そのヒットが構造のどこに属するか**」を返す | 🟢 動く（YAML の部分集合） |

---

## scopegrep — 構造を知る grep

`grep` は「その行がある」ことしか返しません。そして **YAML の入れ子は行番号に現れません。**

CI 設定の中から `cancelled()` を探すとき、本当に知りたいのは
「何行目にあるか」ではなく **「どのステップに付いているか」** です。

同梱の fixture（[`scopegrep-core/testdata/workflow-with-comment.yml`](./scopegrep-core/testdata/workflow-with-comment.yml)）を
`grep` で引くと 5 行返ります。

```console
$ grep -n 'cancelled()' scopegrep-core/testdata/workflow-with-comment.yml
4:#    候補パーサは、下の3つの `cancelled()` を **別物として区別できなければならない**。
29:      # 1) 散文。ここに書かれた cancelled() は設定値ではない。
30:      #    !cancelled() を使う理由を説明しているだけで、実行条件ではない。
33:        if: ${{ !cancelled() }}
46:        if: ${{ !cancelled() }}
```

**うち 3 行（4・29・30）はコメントで、設定値ではありません。**
残る 2 行は同じに見えますが、別物です。

- 33 行目は **依存監査のステップ**（`Audit (fail on high/critical)`）の `if` で、
  前段が落ちた作業ツリーでも走り、意図しない2つ目の赤を出していた（欠陥）
- 46 行目は **Playwright レポートのアップロード**（`Upload Playwright report`）の `if` で、
  E2E が落ちてもレポートを上げるための**教科書どおりの正しい用法**

`grep` の出力からこの違いは読めません。実際に 2026-09-01、同じ形の検索で
「同じ欠陥が2つある」と誤読し、**偽陽性（正しい用法を欠陥と判定）と
偽陰性（ファイル名が違うリポを見落とし）が同時に出ました。**

`scopegrep` は、マッチした値が構造上どこに属するかを返します。
**コメント内の一致は既定では返しません**（上の 5 行に対してこの 2 行です）。

```console
$ scopegrep 'cancelled()' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:33: jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .if = ${{ !cancelled() }}
scopegrep-core/testdata/workflow-with-comment.yml:46: jobs.e2e.steps[2] "Upload Playwright report" .if = ${{ !cancelled() }}
```

コメントを**捨てているのではなく、区別している**ので、`--comments` を付けると
`grep -n` と同じ 5 行が、どちらだったかの印付きで返ります。

```console
$ scopegrep --comments 'cancelled()' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:4: #comment = #    候補パーサは、下の3つの `cancelled()` を **別物として区別できなければならない**。
scopegrep-core/testdata/workflow-with-comment.yml:29: jobs.frontend-check.steps #comment = # 1) 散文。ここに書かれた cancelled() は設定値ではない。
scopegrep-core/testdata/workflow-with-comment.yml:30: jobs.frontend-check.steps #comment = #    !cancelled() を使う理由を説明しているだけで、実行条件ではない。
scopegrep-core/testdata/workflow-with-comment.yml:33: jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .if = ${{ !cancelled() }}
scopegrep-core/testdata/workflow-with-comment.yml:46: jobs.e2e.steps[2] "Upload Playwright report" .if = ${{ !cancelled() }}
```

コメントの所属は「**そのコメントがどの桁に書かれたか**」で決めます。
「このコメントは誰の説明か」は推測しません（29〜30 行目は `steps[3]` の説明ですが、
構文木で持つ実装ではこれが `steps[2]` に付きます。実測は[設計メモ](./docs/design/scopegrep.md)の「D-2 実測」）。

🔴 **この README の `console` ブロックは、実際に実行した出力です。**
`scopegrep/tests/readme.rs` が `make check` のたびにコマンドを実行し、
続く行と**完全一致**することを確かめます（一致しなければテストが落ちます）。

### 設計上の判断

- **コメント内のヒットを既定では返さない。** 行ではなく構造を読むので、
  `# ... cancelled() ...` のような散文と実際の設定値を混同しません。
  行ベースの検索はこれを必ず拾います（上の 5 行と 2 行の差がそれです）。
  区別した結果を捨てているわけではないので、`--comments` を付ければ
  **コメントだと明示した上で**返します
- **読む YAML を部分集合に限り、外はエラーにする。**
  読めるのはブロックマッピング・ブロックシーケンス・1行スカラー（プレーン / `'…'` / `"…"`）・
  ブロックスカラー（`|` `>`）・フロー記法（複数行も可。中には入らず、行ごとに値として持つ）・
  タグ（読み飛ばす）・コメント・先頭の `---` です。
  アンカー・エイリアス・マージキー・複数行のプレーンスカラー・複数ドキュメントは
  **読めません**。黙って誤読した結果を返さず、**何行目の何が読めなかったかを言って落ちます**
  （一覧は[設計メモ](./docs/design/scopegrep.md)の「対応する YAML の部分集合」）
- **部分集合は実測で広げる。** 手元の全リポジトリの `.yml` / `.yaml` 188 ファイルに当てたところ、
  v1 は 169 が読め、GitHub Actions の workflow は 67 本すべて読めました。読めなかった 18 件のうち
  14 件は compose の `healthcheck.test` を複数行に割ったフロー記法、3 件は compose の `!override` `!reset` タグ、
  1 件は `- { $ref: … }` を誤読する**バグ**でした。この 3 つだけを足した v1.1 で 187 / 188 になり、
  残る 1 件は読めないことを確かめるための自前の fixture です。アンカーと複数ドキュメントは
  この 188 ファイルに 0 件だったので、まだ読めません（数字の出どころは設計メモの「実ファイルでの計測」）
- **機械向けの出力を持つ。** `--json` は1ヒット1行の JSON Lines で、
  所属を RFC 6901 の JSON Pointer でも返します。`kind` は `--comments` の有無に
  かかわらず常に出ます（キーの数が入力で変わると、受け手が
  「今回は出ていないだけ」と区別できないため）

```console
$ scopegrep --json 'cancelled()' scopegrep-core/testdata/workflow-with-comment.yml
{"file":"scopegrep-core/testdata/workflow-with-comment.yml","line":33,"column":18,"pointer":"/jobs/frontend-check/steps/3/if","path":"jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if","label":"Audit (fail on high/critical)","value":"${{ !cancelled() }}","kind":"value"}
{"file":"scopegrep-core/testdata/workflow-with-comment.yml","line":46,"column":18,"pointer":"/jobs/e2e/steps/2/if","path":"jobs.e2e.steps[2] \"Upload Playwright report\" .if","label":"Upload Playwright report","value":"${{ !cancelled() }}","kind":"value"}
```

- **終了コードは `grep` と同じ。** 0 = 1件以上ヒット / 1 = ヒット無し / 2 = エラー。
  🔴 **読めないファイルがあれば、ヒットが出ていても 2 で終わります。**
  「一部しか見ていない結果」を成功と呼ばないのが、この道具が生まれた事故への答えです
- **依存は 0。** 中核（`scopegrep-core`）は `#![no_std]` ＋ `alloc` で書かれ、
  時刻・乱数・環境・I/O に**構文的に到達できません**。パーサを自分で書いた理由と、
  候補6件の実測（位置情報・コメントの露出・依存数・`no_std`）は
  [設計メモ](./docs/design/scopegrep.md)の「D-2 実測」節にあります
- **YAML に閉じない。** 同じ問題は TOML / JSON にもあります（v1 には含みません）

### 隣接する既存実装

- [`yamlpath`](https://crates.io/crates/yamlpath) — YAML から値を抽出する（書式を保存する）ライブラリ。
  **「パスを指定して値を取る」方向**で、`scopegrep` の
  **「値を検索してパスを返す」方向**とは逆です
- [`treegrep`](https://crates.io/crates/treegrep) — 検索結果を**ファイルツリー**として表示する。
  返すのはファイル階層であって、**ファイル内の構造ではありません**

---

## 開発

```bash
make check
```

**これが唯一の入口です。** CI も `make check` を呼ぶだけで、CI 側にしか無い検査を作りません
（「手元では通ったのに CI で落ちた」を構造的に起こさないため）。
`make coverage` だけは `cargo-llvm-cov` が要ります（`cargo install cargo-llvm-cov --locked`）。

版は `rust-toolchain.toml` が決めます。`Makefile` にも CI にも版を書きません
（2箇所に書くと、片方だけ上げられて「手元では通る」が生まれるため）。

### 規約

コードの書き方は **[`docs/coding-rules.md`](./docs/coding-rules.md) が正**です。
すべての規則に ID があり、**機械強制の状態（active / planned / 不能 / 不採用）を明示**しています。

思想は「**一つの事を表現する手段を一つに固定する**」ことで、実体は3層です。

| 層 | 守るもの |
| --- | --- |
| コンパイラ / cargo | 型・可視性・網羅性・クレート境界（**不正な状態を書けなくする**） |
| lint | 書けてしまうが書くべきでないこと |
| 規約検査（`xtask`） | xi-tools 固有の規約（依存ゼロ・`make check` に含まれる） |

🔴 **抑制は二段構えです。** `forbid` した規則は `#[allow]` も `#[expect]` も
コンパイルエラー（E0453）になり、**例外を申請する窓口が存在しません**。
`deny` の規則は `#[expect(lint, reason = "...")]` でのみ抑制でき、
不要になった抑制は `unfulfilled_lint_expectations` が落とします。

判断の根拠は [ADR 0001](./docs/adr/0001-strictness-is-mechanically-enforced.md)、
ゲートが実際に発火することの実測は
[ゲート発火の証明](./docs/quality/gate-proofs.md)。

## ライセンス

MIT
