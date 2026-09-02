# xi-tools

[English](./README.md) | 日本語

自分の開発環境のために書いた Rust のツール群。

**汎用ツールとして設計していないものを含みます。** 数十のリポジトリを横断して作業する
個人の環境から出てきた道具なので、前提がその環境に寄っているものがあります。
それでも公開しているのは、**問題の立て方ごと残しておくため**です。

| ツール | 何を解くか | 状態 |
| --- | --- | --- |
| [`scopegrep`](./scopegrep) | grep が返さない「**そのヒットが構造のどこに属するか**」を返す | 🟢 動く（YAML の部分集合） |
| [`fleet-top`](./fleet-top) | **数十のリポジトリの状態を 1 画面で**——枝・未コミット・ahead/behind・open PR・CI・古い枝——「打たれる」時間のうちに | 🟢 動く（60 リポで 1.6〜1.8 秒・実測） |

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

### 使い方

```
scopegrep [-i] [--json] [--comments] [--scope <pattern>] (<needle> | -e <regex>) [<path>...]
```

**`--scope` は所属で絞ります。** 検索語ではなく**構造**で引けるので、
「全ステップの `run` を並べる」のような、`grep` では書けない問いに答えられます
（needle を空にすると、その場所の値が全部並びます）。

```console
$ scopegrep --scope '/jobs/*/steps/*/run' '' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:24: jobs.frontend-check.steps[1] "Install" .run = npm ci
scopegrep-core/testdata/workflow-with-comment.yml:27: jobs.frontend-check.steps[2] "Unit tests" .run = npm test
scopegrep-core/testdata/workflow-with-comment.yml:34: jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .run = npm audit --audit-level=high
scopegrep-core/testdata/workflow-with-comment.yml:42: jobs.e2e.steps[1] "Run Playwright" .run = npx playwright test
```

パターンは出力と同じ JSON Pointer の形で書きます。`*` は**ちょうど1セグメント**、
`**` は**0 個以上**（`/services/**/image` はどの深さの `image` にも当たります）。
それ以外は生のキー／索引と完全一致で、**部分一致のグロブはありません**（`*` の意味を一つに保つため）。
所属パス**全体**との一致を見ます。読めないパターンは黙って直さず、理由を言って終了コード 2 で落ちます。

- **`-i` / `--ignore-case`** — 大文字小文字を無視して照合します。
  列は**原文の一致位置**のままです（小文字化した文字列の上で数えると、
  `İ` のように小文字が2文字になる字を含む行だけ列がずれるため）
- **`-e` / `--regex`** — 固定文字列の代わりに正規表現で探します。`<needle>` とは排他で、
  付けたときは位置引数がすべて `<path>` になります（[使うにはビルド時の指定が要ります](#インストール)）
- **パスの省略** — `scopegrep <needle>` だけで今いる場所を再帰します。
  表示に `./` は付きません。明示的に `.` を渡したときは付きます（`grep -rn x .` と同じ）
- **依存ディレクトリは掘りません** — `.git` `node_modules` `vendor` `target` `.venv`。
  手元の実測（2026-09-02）では自前の `.yml` / `.yaml` が 188 件なのに対し
  `node_modules` 配下に 3,206 件・`vendor` 配下に 3,837 件あり、掘ると出力のほぼ全部が
  他人のファイルになります。**名指しされたパスは除外しません**（`scopegrep x node_modules/foo/` は読みます）

### 正規表現は opt-in です

`-e` / `--regex` は**既定のビルドには入っていません**。この道具の前提は「単一バイナリで配れる・
中核は依存 0」なので、`regex` crate（推移的に 3 件）を要る人にだけ渡す形にしました
（判断の根拠は [ADR 0002](./docs/adr/0002-regex-is-an-opt-in-feature.md)）。
所属で絞る `--scope` とは独立に効きます。

```console
$ scopegrep -e 'npm (ci|test)' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:24: jobs.frontend-check.steps[1] "Install" .run = npm ci
scopegrep-core/testdata/workflow-with-comment.yml:27: jobs.frontend-check.steps[2] "Unit tests" .run = npm test
```

一致は**行単位**です（`^` `$` は行の先頭と末尾で、複数行スカラーを跨ぐ一致はしません）。
値を行ごとに持つ設計の帰結です。`-i` は正規表現側では `RegexBuilder::case_insensitive` で渡すので、
固定文字列の1文字ずつの case fold とは Unicode の扱いが微妙に違います。

🔴 **正規表現なしでビルドした binary で `-e` を打つと、終了コード 2 で
「この binary は正規表現なしでビルドされている」と言って落ちます。**
黙って固定文字列として扱いません。どちらのビルドかは `scopegrep --version` が
`(regex: on)` / `(regex: off)` で返します。

### インストール

```bash
cargo install scopegrep                    # 固定文字列の検索。依存 0
cargo install scopegrep --features regex   # -e / --regex が付く（3 crate: regex・regex-automata・regex-syntax）
```

各 OS の binary（正規表現入り）は
[GitHub Releases](https://github.com/hideyukiMORI/xi-tools/releases) にあります。

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
- **既定ビルドの依存は 0。** 中核（`scopegrep-core`）は `#![no_std]` ＋ `alloc` で書かれ、
  時刻・乱数・環境・I/O に**構文的に到達できません**。パーサを自分で書いた理由と、
  候補6件の実測（位置情報・コメントの露出・依存数・`no_std`）は
  [設計メモ](./docs/design/scopegrep.md)の「D-2 実測」節にあります。
  唯一の例外が opt-in の `regex` で、**中核ではなくバイナリ側に入ります**
  （中核は照合を `Matcher` trait で受け取るだけで、正規表現を知りません）。
  ライセンス・脆弱性・重複バージョン・取得元は `make deny`（`cargo-deny`）が見ます
- **YAML に閉じない。** 同じ問題は TOML / JSON にもあります（v1 には含みません）

### 隣接する既存実装

- [`yamlpath`](https://crates.io/crates/yamlpath) — YAML から値を抽出する（書式を保存する）ライブラリ。
  **「パスを指定して値を取る」方向**で、`scopegrep` の
  **「値を検索してパスを返す」方向**とは逆です
- [`treegrep`](https://crates.io/crates/treegrep) — 検索結果を**ファイルツリー**として表示する。
  返すのはファイル階層であって、**ファイル内の構造ではありません**

---

## fleet-top — 数十のリポジトリの状態を 1 画面で

約 60 の git リポジトリを並べて作業しています。1 日に何度も同じことを知りたくなります。
**全リポについて、今どの枝にいて、未コミットがあり、上流とどれだけずれ、open な PR があり、CI は緑で、古い枝は残っていないか。**
それは毎回その場で書き捨てのシェルのループになり、そしてそのループは遅かった。

2026-09-01 の実測: `gh api` は 1 本 0.67〜0.74 秒。リポごとに 3 本（設定・open PR・CI）を 42 リポで 126 本、
直列で **約 84〜93 秒**。

🔴 **84 秒かかるコマンドは打たれません。** 打たれないので、見えるはずのもの——期限切れ・4 日間だれも見なかった監査——が
見えないままになる。この道具の目的は「速くて気持ちいい」ではなく、**打たれるコマンドと打たれないコマンドの境界を越えること**です。

```text
$ fleet-top ~/docker
REPO                                  BRANCH                                  DIRTY  AHEAD/BEHIND  PR   CI    STALE
NENE2                                 main                                    -      -             10   ok    -
NENE2-examples-repo                   main                                    -      -             -    -     -
NeNe                                  main                                    -      -             -    ok    ?
_work                                 main                                    -      -             -    -     -
eventlog                              docs/ft13-milestone                     -      (none)        n/a  n/a   n/a
gtypist-lesson                        master                                  -      -             -    -     -
hideyuki-mori-site                    main                                    -      -             -    -     -
hideyukiMORI                          master                                  1      (none)        n/a  n/a   n/a
hoplog                                main                                    -      -             -    -     -
keyquest                              main                                    -      -             -    -     -
knowledgelog                          main                                    -      (none)        n/a  n/a   n/a
…（残り 49 行）
fleet-top: NeNe: 枝が 100 本を超えている。STALE は数えていない
fleet-top: 60 repos, 45 on GitHub, 1.6s
```

2026-09-02 に自分のディレクトリで取った実出力です（60 行のうち先頭 11 行。最後の 2 行は stderr）。
上の `scopegrep` の例と違い、**このブロックはテストで照合していません**——出力がその時点の GitHub と作業木の状態に依存するからです。
照合しているのは整形のほうで、`fleet-top-core` の fixture テストが表を 1 文字単位で見ています。

表の読み方:

- `-` はゼロ・該当なし。`n/a` は `origin` が GitHub でないリポ（聞いていない）。`?` は取れなかった値——**`?` には必ず stderr に理由が 1 行**つき、
  行は消えません。失敗した行を消すのは、この道具が防ぎたい事故（たまたま見えた片方で判断する）と同じ形です
- 終了コードは、全行が確定なら 0、`?` があれば 1（表は出ています）、使い方の誤りやディレクトリが読めなければ 2
- `AHEAD/BEHIND` は手元にある追跡枝との差です。**`git fetch` は打ちません**。見るだけです

### 使い方

```
fleet-top [DIR] [--stale-days N] [--no-github]
```

`DIR` の既定は `.`。その直下で `.git` を持つものだけがリポジトリで、再帰しません。`--stale-days`（既定 30）は
GitHub 上の既定枝以外の枝を「古い」と呼ぶまでの日数。`--no-github` は `gh` を起動せず、GitHub の 3 列を `n/a` にします。

GitHub は `gh api graphql` 経由で読むので、**`gh` が入っていてログイン済みであること**が前提です。認証を借りるだけで、token は扱いません。

### インストール

crates.io にはまだ出していません。リポジトリから:

```bash
cargo install --path fleet-top
```

### 作る前に測ったこと

設計は想定ではなく 1 時間の試作で決めました（全表は [`docs/benchmarks/fleet-top.md`](./docs/benchmarks/fleet-top.md)）。

| 形 | 60 リポ |
| --- | --- |
| REST 直列（21 本 0.74 s/本からの外挿） | 93 s |
| REST 64 並列 | 2.38 s・rate limit 126 点 |
| GraphQL **1 本**に全リポ | 42 リポで 8.87 s、60 リポで **HTTP 502** |
| GraphQL **3 リポ × 1 本を全部並列** | **1.35〜1.49 s**・20 点 |
| 道具そのもの（release ビルド・60 リポ中 45 が GitHub） | **1.6〜1.8 s** |

予想していなかった結果は、**GraphQL は 1 本にまとめるほど遅くなる**ことでした。60 リポを 1 本にすると返ってきません。
小さく割って一斉に投げるほうが、1 本にまとめるより、REST をどれだけ並列にするより速かった。core の `REPOS_PER_QUERY = 3` はこの表からそのまま来ています。

### 実際に起きた失敗

- **`gh api graphql` は、リクエスト内の 1 リポが失敗すると終了コード 1 を返し**、他のリポのデータは stdout に載ったままです。
  終了コードで判断していたら、存在しない 1 リポのために 3 リポ分を捨てていました。道具は終了コードを見ずに stdout を読み、
  `errors[].path` が指すリポだけを失敗にします
- **設計メモの出力例が、設計メモ自身の規則と矛盾していました**（バイト順でない並び・「読めなかった」のに枝名がある行）。
  実装側の完全一致 fixture テストが拾い、メモを直しました
- **最初の実機実行で、理由の無い `?` が 1 つ出ました。** 枝が 100 本を超えるリポの古い枝の数は切り詰めで数えられず、
  失敗ではないので stderr に何も出ていなかった。今は理由を出し、テストで押さえています

### 設計上の判断

却下した案とあわせて [`docs/design/fleet-top.md`](./docs/design/fleet-top.md) と
[ADR 0003](./docs/adr/0003-fleet-top-fetches-github-via-chunked-graphql.md) に記録しています。

- **両クレートとも依存 0。** GitHub は `gh` のサブプロセス（認証を借りる）、並列は `std::thread::scope` と上限つきのワーカープール、
  GraphQL 応答の JSON は `fleet-top-core` に手書きした RFC 8259 パーサで読みます。`serde_json` は 5 crate 入り、`no_std` の中核が崩れます
- **I/O が本体の道具でも中核は `no_std`。** `git` と `gh` の出力を文字列として、「今日」を値として受け取り、表を返す。
  プロセスの起動・待ち合わせ・時計は bin に残す。中核の全部が fixture だけでテストできます
- **TUI も `--watch` も作らない。** 1.6 秒で返るものを常駐させる理由が無い（`watch fleet-top` がある）。`ratatui` は数十 crate 入ります
- **見るだけ。** `fetch` も `checkout` も `merge` もしません

## 開発

```bash
make check
```

**これが唯一の入口です。** CI も `make check` を呼ぶだけで、CI 側にしか無い検査を作りません
（「手元では通ったのに CI で落ちた」を構造的に起こさないため）。
道具が要るのは2つだけです——`make coverage` は `cargo-llvm-cov`、
`make deny` は `cargo-deny`（どちらも `cargo install <name> --locked`）。

`make check` は**両方の構成**（既定と `--features scopegrep/regex`）で lint とテストを回します。
片方だけ緑の状態を作らないためです。

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
