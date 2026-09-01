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
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

版は `rust-toolchain.toml` が決めます。CI にも版を書きません
（2箇所に書くと、片方だけ上げられて「手元では通る」が生まれるため）。

## ライセンス

MIT
