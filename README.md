# xi-tools

自分の開発環境のために書いた Rust のツール群。

**汎用ツールとして設計していないものを含みます。** 数十のリポジトリを横断して作業する
個人の環境から出てきた道具なので、前提がその環境に寄っているものがあります。
それでも公開しているのは、**問題の立て方ごと残しておくため**です。

| ツール | 何を解くか | 状態 |
| --- | --- | --- |
| [`scopegrep`](./scopegrep) | grep が返さない「**そのヒットが構造のどこに属するか**」を返す | 🔲 設計中 |

---

## scopegrep — 構造を知る grep

`grep` は「その行がある」ことしか返しません。そして **YAML の入れ子は行番号に現れません。**

CI 設定の中から `cancelled()` を探すとき、本当に知りたいのは
「何行目にあるか」ではなく **「どのステップに付いているか」** です。

```console
$ grep -n 'cancelled()' */.github/workflows/frontend-ci.yml
nene-corpus/...:66:        if: ${{ !cancelled() }}
nene-profile/...:106:       if: ${{ !cancelled() }}
```

この2行は同じに見えます。実際には:

- 一方は **依存監査ステップ**に付いていて、前段が落ちた作業ツリーで走り、
  意図しない2つ目の赤を出していた（欠陥）
- もう一方は **Playwright レポートのアップロード**に付いていて、
  E2E が落ちてもレポートを上げるための**教科書どおりの正しい用法**

`grep` の出力からこの違いは読めません。実際にこれを「同じ欠陥が2つある」と誤読し、
**偽陽性（正しい用法を欠陥と判定）と偽陰性（ファイル名が違うリポを見落とし）が同時に出ました。**

`scopegrep` はマッチした行が属する構造上の位置を返します。

```console
$ scopegrep 'cancelled()' .github/workflows/ --show-scope
corpus/frontend-ci.yml   jobs.frontend-check.steps[6] "Audit (fail on high/critical)" .if
profile/frontend-ci.yml  jobs.e2e.steps[5]            "Upload Playwright report"      .if
```

### 設計上の判断

- **コメント内のヒットを区別する。** 構文木で読むので、`# ここで cancelled() を使う理由` の
  ような散文と、実際の設定値を混同しません（行ベースの検索はこれを必ず拾います）
- **YAML に閉じない。** 同じ問題は TOML / JSON にもあります

詳細は [`docs/design/scopegrep.md`](./docs/design/scopegrep.md)。

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
| 規約検査（`xtask`） | xi-tools 固有の規約（**未実装**） |

🔴 **抑制は二段構えです。** `forbid` した規則は `#[allow]` も `#[expect]` も
コンパイルエラー（E0453）になり、**例外を申請する窓口が存在しません**。
`deny` の規則は `#[expect(lint, reason = "...")]` でのみ抑制でき、
不要になった抑制は `unfulfilled_lint_expectations` が落とします。

判断の根拠は [ADR 0001](./docs/adr/0001-strictness-is-mechanically-enforced.md)、
ゲートが実際に発火することの実測は
[ゲート発火の証明](./docs/quality/gate-proofs.md)。

## ライセンス

MIT
