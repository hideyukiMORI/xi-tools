# Rust コーディング規約 — xi-tools

> Status: normative（規範）/ 2026-09-02 初版
> 判断の根拠は [ADR 0001](adr/0001-strictness-is-mechanically-enforced.md)。
> 本書は Rust を本リポジトリで承認された部分集合に狭めるためのものであり、
> ここが沈黙している領域は公式の Rust スタイル（Rust API Guidelines / rustfmt の既定）に従う。

---

## 0. この文書の読み方

**すべての規則は「機械強制」の状態を持つ。**

| 表記 | 意味 |
| --- | --- |
| **active** | 違反すると `make check` が落ちる。人の記憶に依存しない |
| **planned** | 規範だが、まだ機械が見ていない。**この行に触れる変更は PR で明示的に自己レビューすること** |
| **不能** | 言語・道具の仕様上、機械では塞げない。塞げないことを明記して残す |
| **不採用** | 検討して採らなかった。理由を必ず併記する（再提案は同じ理由への反論から始める） |

🔴 **planned を active と書き換えないこと。** 未実装の強制を実装済みに見せるのは、
規約全体の信頼を壊す唯一の行為である。実装してから書き換える。

### 強制の実体は3層ある

| 層 | 実体 | 守るもの |
| --- | --- | --- |
| コンパイラ / cargo | 型・可視性・網羅性・クレート境界 | **不正な状態を「書けなく」する** |
| lint | `Cargo.toml` の `[workspace.lints]` ＋ `clippy.toml` | 書けてしまうが書くべきでないこと |
| 規約検査 | `xtask`（依存ゼロ・`make check` に含まれる） | **xi-tools として守るべきこと** |

### 🔴 抑制は二段構えである（本規約の中核）

Rust の lint には `forbid` という水準があり、**`#[allow]` も `#[expect]` も
コンパイルエラー（E0453）にする**。これが本規約の設計の中心にある。

| 段 | 水準 | 抑制 | 使う場面 |
| --- | --- | --- | --- |
| **1段目** | `forbid` | **不可能**。例外を申請する窓口が存在しない | 例外を認めた瞬間に規則の意味が消えるもの |
| **2段目** | `deny` | `#[expect(lint, reason = "...")]` **のみ** | 「唯一の許可された場所」を宣言させたいもの |

2段目には仕掛けがある。

- `clippy::allow_attributes` が **forbid** なので、`#[allow]` は書けない。抑制は `#[expect]` だけ
- `clippy::allow_attributes_without_reason` が **forbid** なので、理由の無い抑制は書けない
- `unfulfilled_lint_expectations` が **deny** なので、**発火しなくなった `#[expect]` は失敗する**

⇒ 抑制は「必ず理由付きで」「必ず今も必要で」しか存在できない。
**古い抑制が黙って腐り続ける経路が無い。**

🔑 これは NENE-PIXEL の `KOT-022`（waiver 制度）が人手の手続きで達成していたものを、
**コンパイラが代わりに執行している状態**である。だから本リポジトリは waiver 台帳を持たない
（判断の根拠は ADR 0001 の「却下した選択肢」）。

### Rust がどこまで届いたか

先行事例（NENE-PIXEL / nene-recall）が言語で強制できなかった4点の実測結果。
測定手順と出力は [ゲート発火の証明](quality/gate-proofs.md)。

| 規則 | Kotlin | Go | **Rust（実測）** |
| --- | --- | --- | --- |
| 不正状態を表現不能に・網羅性 | ある | **無い** | **ある**（`E0004`。`_` は `wildcard_enum_match_arm` が forbid） |
| 公開状態は不変 | ある | **無い** | **ある**（`let` が既定で不変。可変性が型に出る） |
| `null` の意味は一つ | 不十分 | 不十分 | **ある**（`null` が無い。ゼロ値も無い＝`E0381`） |
| private コンストラクタ＋唯一のファクトリ | ある | **不十分** | **ある**（`E0603`。ただし RS-003 に穴が1つ残る） |
| 抑制は例外であって道具ではない | waiver 台帳（人手） | 理由必須どまり | **より強い**（forbid 層は抑制不能・deny 層は腐らない） |

🔴 **それでも「不正な状態を表現不能にする」は完全ではない。** 残る穴は RS-003 と
第7節に明記した。**穴が残ることを書かずに厳格さを主張しないこと。**

---

## 1. 型と状態（RS-0xx）

### RS-001 — 境界でのプリミティブ執着を禁じる

識別子・オフセット・行番号・深さ・パスのように**単位や不変条件を持つ値**は、
非公開フィールドを持つ newtype にする。

```rust
pub struct LineNumber(u32);          // ✅ フィールドが非公開

impl LineNumber {
    /// 生成経路はここだけ。0 行目は存在しないので拒否する。
    pub fn new(v: u32) -> Option<Self> { (v > 0).then_some(Self(v)) }
}
```

🔑 **Go と決定的に違う点。** `LineNumber(1)` はモジュールの外では `E0603`
（実測）。Go では `org.ID(1)` を検査器（`CNF-001`）で禁じるほか無かったが、
Rust ではコンパイラが拒否する。**唯一のファクトリを言語が保証する。**

- 機械強制: **active**（コンパイラ `E0603`。newtype にするかどうかの判断は **planned**）

### RS-002 — 閉じた選択肢は enum で表す。`_` を書かない

モードや状態機械を bool の組み合わせ・マジック整数・裸の文字列で表さない。

`match` の網羅性はコンパイラが見る（`E0004`）。したがって**規約が守るべきなのは
「網羅性検査を無効化しないこと」**である。`_` を書いた瞬間に検査は死ぬ。

```rust
match kind {
    NodeKind::Mapping  => ...,
    NodeKind::Sequence => ...,
    _ => ...,            // ❌ 将来足した variant を黙って飲む
}
```

- 機械強制: **active**（`E0004` ＋ `clippy::wildcard_enum_match_arm` / `wildcard_in_or_patterns` を **forbid**）
- 補足: 自クレートの enum に `#[non_exhaustive]` を付けないこと。付けると
  下流に `_` を強制し、この規則を自分で壊す

### RS-003 — 不変条件を持つ型に「作り置きの値」を用意しない

🔴 **Rust に残る唯一の実質的な穴がここにある。**

Go の `var id org.ID` に相当するものは Rust には無い。未初期化の変数を使えば
`E0381` で落ち、構造体の部分構築は `E0063` で落ちる（どちらも実測）。
**しかし `#[derive(Default)]` を書けば、自分でゼロ値を作れてしまう。**

```rust
#[derive(Default)]      // ❌ 不変条件を持つ型に書かない。ファクトリを迂回する門を自分で開ける
pub struct LineNumber(u32);
```

したがって次を規範とする。

- **`Default` を実装・derive しない**（型を問わず）。
  🔑 「不変条件を持つ型に限る」と書くと、機械が判定できない。
  **判定できない条件を規約に書くと、その規則は永久に planned のままになる。**
  全面禁止にして、必要になったら ADR で例外を切る
- `..Default::default()` による構造体の穴埋めを書かない（`E0063` を無効化する書き方である）
- 不変条件を持つ型は**自分のモジュールに単独で置く**。
  🔑 Rust の可視性は「子モジュールは祖先の非公開項目に到達できる」。
  同じモジュールに雑多な型を同居させると、隣人がファクトリを迂回できる

- 機械強制: **active**（未初期化 `E0381`・部分構築 `E0063`・`Default` の禁止＝`CNF-001`）
- 機械強制: **planned**（単独モジュール＝`CNF-005`）

### RS-004 — 不在は `Option` だけが表す

`Option::None` は「省略可能な値が無い」だけを意味する。
無効・未読込・失敗・未知・削除済みを表さない。それらは専用の enum を作る。

- `Option<Result<T, E>>` と `Result<Option<T>, E>` を場当たりに混ぜない。
  どちらを使うかを型ごとに決めて、その型では一つに固定する
- `unwrap` / `expect` を書かない（RS-005）

- 機械強制: **active**（言語に `null` が無い。`unwrap_used` / `expect_used` を **forbid**）
- 機械強制: **planned**（`Option` の意味の流用そのものはレビュー事項）

### RS-005 — 期待される失敗は `Result`。panic しない

検証エラー・見つからない・拒否・非互換は `Result` で返す。

**禁止（forbid・抑制不可）**: `unwrap` / `expect` / `panic!` / `unreachable!` /
`v[i]` 添字アクセス / `std::process::exit` / `mem::forget`。

```rust
let node = nodes.get(i).ok_or(ScopeError::MissingNode)?;   // ✅
let node = nodes[i];                                        // ❌ indexing_slicing
```

- **テストコードは対象外**（`clippy.toml` の `allow-*-in-tests`）。
  テストでは「失敗したら即座に落ちる」ことが正しい振る舞いであり、
  ここを禁じるとテストが本質でない `Result` 処理で埋まる。
  🔴 **免除は `clippy.toml` で行い、テスト側に `#[expect]` を書かせない。**
  書かせると抑制が常態化し、二段構えの意味が消える
- `std::process::exit` は `main` の中では clippy が見逃す（clippy の仕様）。
  終了コードは `main` から返す形に寄せること

- 機械強制: **active**（上記 forbid 群 ＋ `clippy::panic_in_result_fn` / `unwrap_in_result`）

### RS-006 — 汎用データバッグを禁じる

`dyn Any`・`BTreeMap<String, String>` を型の代用にしない。意味のある値を
`(A, B)` のタプルで持ち回らない。名前付きの型を作る。

- 機械強制: **active**（`dyn Any` の禁止＝`CNF-002`）
- 機械強制: **planned**（文字列キーのバッグ・意味を持つタプルの検出）

### RS-007 — 数値変換に `as` を使わない

`as` は黙って切り詰める。`From` / `TryFrom` のどちらか一つを使う。

```rust
let n: u32 = u32::try_from(len)?;   // ✅
let n = len as u32;                  // ❌ as_conversions（forbid）
```

- 機械強制: **active**（`clippy::as_conversions` を **forbid**）

---

## 2. 構築と可視性

### RS-008 — 可視性は最小

- 既定は非公開。`pub(crate)` で足りるものを `pub` にしない
- 公開項目には doc コメントを付ける（`missing_docs`）
- 構造体のフィールドは全公開か全非公開。混在させない（`partial_pub_fields`）
- フィールドに `pub(crate)` 等のスコープ修飾子を付けない（`field_scoped_visibility_modifiers`）。
  不変条件を守る型のフィールドは非公開であり、公開したいならメソッドを生やす
- 再エクスポート（`pub use`）を書かない。**同じ物に二つ目の名前を作る行為**である

- 機械強制: **active**（`unreachable_pub`・`missing_docs`・`partial_pub_fields`・
  `field_scoped_visibility_modifiers`・`clippy::pub_use`）

### RS-009 — 可変グローバルを持たない

パッケージ全体で共有される可変状態を作らない。初期化は `main` の配線点で明示的に行う。

- `static mut` は `unsafe` を要するので、`unsafe_code = "forbid"` により**書けない**
- `OnceLock` / `LazyLock` による遅延グローバルは書けてしまう。使わない

- 機械強制: **active**（`static mut` は `unsafe_code` の forbid による。
  `OnceLock` / `LazyLock` / `lazy_static` の禁止＝`CNF-002`）

### RS-010 — 言語マジックを禁じる

- `unsafe` を書かない
- 手続きマクロ（proc-macro）を自作しない。使うときは ADR
- `build.rs` を書かない。ビルド時にコードを生成しない
- 宣言的マクロ（`macro_rules!`）は、同じ形の重複が3箇所以上あり、
  かつ関数と generics で書けないときに限る

🔑 Rust には reflection が無い。Kotlin / PHP の `KOT-010` が禁じていたものの
大半が**そもそも言語に存在しない**。

- 機械強制: **active**（`unsafe_code` を **forbid**。reflection は言語に無い。
  proc-macro の禁止＝`CNF-002`。**`build.rs` の禁止＝`CNF-008`**）
- 機械強制: **planned**（`macro_rules!` の判断はレビュー事項）

### RS-011 — 複雑度に上限を置く

| 指標 | 上限 | lint |
| --- | --- | --- |
| 認知的複雑度（関数） | 10 | `clippy::cognitive_complexity` |
| 関数の長さ | 60 行 | `clippy::too_many_lines` |
| ネストの深さ | 4 | `clippy::excessive_nesting` |
| 引数の数 | 4 | `clippy::too_many_arguments` |
| 型の複雑さ | 250 | `clippy::type_complexity` |

閾値を満たすためだけに意味のある処理を割るのは目的に反する。
超える必要があるときは**測定可能な理由**を添えて ADR にする。

🔴 **`clippy.toml` に閾値だけ書いても効かない lint がある。**
`cognitive_complexity` は `nursery` 群にあり、どの既定群にも入らない。
`Cargo.toml` 側で明示的に有効化していなければ、**閾値は死んだ設定になる**（実測で確認済み）。
閾値を足すときは必ず lint の有効化も確認すること。

- 機械強制: **active**

### RS-012 — ファイルとモジュールの置き方を一つに固定する

- `mod.rs` を作らない。`foo.rs` ＋ `foo/` の形だけを使う
- 1ファイルに主要な宣言は1つ。`utils.rs` のような寄せ集めを作らない

🔴 `clippy::mod_module_files` と `clippy::self_named_module_files` は
**互いに正反対の規則**である（実測: 一方は `mod.rs` を禁じ、他方は要求する）。
**同時に有効化しないこと。** 本リポジトリは前者を採る。

- 機械強制: **active**（`mod.rs` の禁止＝`clippy::mod_module_files`、
  1ファイル1主要宣言＝`CNF-003`）

### RS-013 — 名前が役割を語る

常に禁止する型名の語尾: `Manager` / `Helper` / `Util` / `Utils` / `Common`。
常に禁止するモジュール名: `utils` / `helpers` / `managers` / `misc` / `common`。

`Processor` や `Data` のように**文脈次第で妥当な語は機械では拒否しない**。
機械が拒否してよいのは「常に禁止」だけで、判断が要る語はレビューの仕事である。

- 機械強制: **active**（束縛名は `clippy.toml` の `disallowed-names`、
  型名の語尾とモジュール名は `CNF-004`）

---

## 3. 実行時の規律

### RS-014 — 出力は1箇所に集約する

標準出力・標準エラーに書いてよいのは、出力を担当する唯一のモジュールだけ。

🔑 **これが二段構えの2段目の使い方の見本である。**
`print_stdout` / `print_stderr` は `deny` にしてあるので、
許可された場所は `#[expect]` で自己申告することになる。

```rust
#[expect(clippy::print_stderr, reason = "RS-014: 出力は1箇所に集約する")]
fn main() { ... }
```

⇒ **「どこが出力してよい場所か」がコードを grep すれば全部出る。**
そして出力をやめたらその `#[expect]` は `unfulfilled_lint_expectations` で落ちる。

- 機械強制: **active**（`clippy::print_stdout` / `print_stderr` / `dbg_macro`）

### RS-015 — プロセス環境に触るのは配線点だけ

環境変数・コマンドライン引数・標準入出力・カレントディレクトリを読んでよいのは
バイナリクレート（`main`）だけ。ライブラリ側は値として受け取る。

- 機械強制: **active**（`scopegrep-core` が `#![no_std]` なので、そこから
  `std::fs` / `std::env` / 標準入出力は**名前解決エラー**になる
  （実測: [C-9](quality/gate-proofs.md)）。宣言が消えていないことは `CNF-007` が見る）

### RS-016 — 反復順は決定的

`HashMap` / `HashSet` を使わない。**反復順が実行ごとに変わり、出力が再現しなくなる。**
`BTreeMap` / `BTreeSet` を使う。

検索ツールの出力が実行ごとに並び替わると、差分が取れず、CI で比較できない。

- 機械強制: **active**（`clippy.toml` の `disallowed-types` ＋ `clippy::iter_over_hash_type`）

### RS-017 — 整数のオーバーフローを黙って巻き戻さない

リリースビルドでも `overflow-checks = true` にする。
既定ではオーバーフローが黙って巻き戻り、**デバッグビルドでだけ落ちる**という
最悪の形の非再現バグになる。

- 機械強制: **active**（`Cargo.toml` の `[profile.release]`）

### RS-018 — 数値リテラルは型を明示する

型注釈の無い数値リテラルは `i32` に落ちる。

🔑 **Go の「未型付き定数が暗黙変換される」穴（nene-recall ADR 0010 の実測）に対応する規則。**
Rust では既定型への落下として現れるので、それを検出できる。

- 機械強制: **active**（`clippy::default_numeric_fallback`）

---

## 4. アーキテクチャ（ARC-0xx）

### ARC-001 — 1ツール = 1クレート

`xi-tools` はツールを並べる workspace である。ツールは1つのクレートに閉じる。
**ツール間でコードを共有したくなったら ADR を1本立てる**（共有は結合であり、
「道具を独立に捨てられる」という本リポジトリの性質を失う）。

**道具クレートは成果物ではない。** `xtask`（規約検査）は `publish = false` を持ち、
このリポジトリを検査するためだけに存在する。ツールの本数には数えない。

🔑 各ツールは `cargo publish -p <name>` で単独 publish できる形を保つ
（`scopegrep` を将来独立させる余地を潰さないため）。

- 機械強制: **planned**（現在ツールは `scopegrep` 1つで、共有クレートが存在しない）

### ARC-002 — 層はクレートで表す

ツール内部を層に分けるなら、モジュールではなく**クレート**に分ける。

🔑 **Rust では依存の方向がビルドシステムに強制される。**
各クレートの `[dependencies]` が**そのまま import 可能なものの全リスト**であり、
循環は cargo が拒否する（実測: `error: cyclic package dependency`）。
Go の `depguard` のような lint を書く必要が無い。**設定ではなく構造で守られる。**

- 機械強制: **active**（cargo。`scopegrep` → `scopegrep-core` の一方向で、
  循環は cargo が拒む）

### ARC-003 — 中核は `no_std` で純粋性を表明する

解析の中核（構文木を作る部分）は `#![no_std]` ＋ `extern crate alloc` で書く。

🔑 **これが Rust における pure-core の実装である。**
`no_std` にすると `std::time`・`std::fs`・`std::env`・`std::io` は
lint 違反ではなく**名前解決エラー**になる
（実測: 式で書けば `E0433: cannot find module or crate std`、
`use std::fs;` と書けば `E0432: unresolved import std`）。
時刻・乱数・環境・I/O が**構文的に到達不能**になるので、決定性が構造で保証される。

- 機械強制: **active**（`scopegrep-core` が `#![no_std]`。到達不能性はコンパイラが守り
  （実測: [C-9](quality/gate-proofs.md)）、**宣言そのものが消えていないこと**は `CNF-007` が見る。
  🔑 宣言が消えれば `std` が黙って戻るので、コンパイラだけでは片手落ちである）

### ARC-004 — 外部依存は許可制

- 依存を足すときは ADR を1本立てる。**現在の依存は 0 である**
- バージョンに `*` を書かない（`clippy::wildcard_dependencies`）
- `Cargo.lock` をコミットする。ゲートは `--locked` で走り、
  lock が更新される状態でのゲート通過を拒む

- 機械強制: **active**（`wildcard_dependencies`・`--locked`）
- 機械強制: **planned**（ライセンス・脆弱性・重複バージョンの検査＝`cargo-deny` の導入）

---

## 5. ゲートの健全性（QLT-0xx）

### QLT-001 — 警告はエラー

`rustfmt` 差分・clippy の指摘・rustdoc の警告は CI を落とす。

- 機械強制: **active**（`make check` の各ターゲット。clippy は `-D warnings`）

### QLT-002 — baseline を持たない

既存違反を凍結する仕組みを使わない。

🔑 Rust には golangci-lint の `new-from-rev` に相当する機構が無い。
baseline の代用になりうるのは**クレート単位の `#![allow]`** だけで、
それは `clippy::allow_attributes` が **forbid** なので書けない。
**baseline を作る経路が言語側で塞がっている。**

- 機械強制: **active**

### QLT-003 — ローカルと CI は同一

`make check` が唯一の入口。CI は `make check` を呼ぶだけで、
CI 側にしか無い検査を作らない。検査を足したくなったら、まず `Makefile` に足す。

- 機械強制: **active**（`.github/workflows/ci.yml` は `make check` の1行のみ）

### QLT-004 — 生成物の差分は失敗

`cargo fmt --all --check` に差分が出る状態、
`--locked` で `Cargo.lock` が更新される状態でコミットしない。

🔴 **`rustfmt.toml` を置かない。** 整形の流儀を議論する余地をそもそも作らない。

- 機械強制: **active**

### QLT-005 — ゲートを弱める変更は ADR

閾値の緩和・`forbid` から `deny` への降格・lint の削除・`clippy.toml` の免除追加・
規則の降格が対象。CI は `Cargo.toml` / `clippy.toml` / `rust-toolchain.toml` /
`Makefile` / `ci.yml` / 本書の差分を PR に通知する。

- 機械強制: **active**（`gate-change-notice` ジョブ）

### QLT-006 — 抑制の規律

第0節の二段構えに従う。加えて:

- 抑制は**最小のスコープ**に付ける。クレート単位・モジュール単位で書かない
- `reason` には**規則 ID を書く**（`reason = "RS-014: ..."`）。
  「なぜ抑制したか」ではなく「どの規則の、どの例外か」を残す
- `TODO` / `FIXME` は Issue 番号を伴うか、消す

- 機械強制: **active**（forbid 層 ＋ `allow_attributes` ＋ `allow_attributes_without_reason`
  ＋ `unfulfilled_lint_expectations` ＋ **`reason` が実在する規則 ID を引くこと**＝`CNF-006`）
- 機械強制: **planned**（`TODO` / `FIXME` の Issue 番号）

### QLT-007 — テストと検査器の規律

- 振る舞いの変更にはテストを付ける。修正は可能なら**先に落ちるテスト**を書く
- **ゲートを足したら、意図的な違反で発火することを証明する。**
  記録先は [`docs/quality/gate-proofs.md`](quality/gate-proofs.md)

🔴 **検査の最大の失敗は、見逃したまま常に緑を返すことである。**
本物のコードを見ている限り、それは永久に発覚しない。
実際、本規約の初版でも `cognitive_complexity` が
**閾値だけ設定されて lint が無効という死んだ状態**だったことが、
この証明手順で見つかっている。

- 機械強制: **active**（証明の存在は人手。手順と実出力は上記文書）

### QLT-008 — カバレッジの下限

実装が入った時点で `cargo-llvm-cov` の下限を `Makefile` に置き、
**上げる方向にしか動かさない。**

- 機械強制: **planned**（実装は入ったが、下限をまだ `Makefile` に置いていない。
  🔴 下限を置くまで active と書かないこと。数字を測っていない段階で
  カバレッジを主張しないこと）

### QLT-009 — 性能の主張は実測を伴う

「速い」は測定結果を伴わなければ主張しない。記録先は `docs/benchmarks/`。
`grep` との比較を主張するなら、対象・入力サイズ・回数を明記する。

- 機械強制: **planned**

---

## 6. 規約検査（CNF-0xx）— `xtask`

lint が見ないもの、つまり**このリポジトリ固有の規約**を検査する。
依存ゼロで書かれており、`make check` の `conformance` ターゲットで常に走る。

| ID | 内容 | 状態 |
| --- | --- | --- |
| `CNF-001` | `Default` の実装・derive・`..Default::default()` を禁じる（RS-003） | **active** |
| `CNF-002` | `dyn Any`・`OnceLock`/`LazyLock`/`lazy_static`・proc-macro を禁じる（RS-006 / RS-009 / RS-010） | **active** |
| `CNF-003` | 1ファイル1主要宣言（RS-012） | **active** |
| `CNF-004` | 役割を語らない型名の語尾・モジュール名を禁じる（RS-013） | **active** |
| `CNF-005` | 不変条件を持つ型は自分のモジュールに単独で置く（RS-003） | planned |
| `CNF-006` | `#[expect]` の `reason` が**実在する規則 ID**を引く／文書内リンクが実在する（QLT-006） | **active** |
| `CNF-007` | 名前が `-core` で終わるクレートは、先頭の属性に `#![no_std]` を宣言する（ARC-003 / RS-015） | **active** |
| `CNF-008` | `build.rs` を置かない／マニフェストから指さない（RS-010） | **active** |

### 検査対象と、その境界

🔴 **`.rs` は桁 0 の `#[cfg(test)]` が現れた行より後を見ない。**
テストは意図的な違反を書く場所であり、そこを検査すると
**検査器のテストが検査器自身に落とされる**。テストモジュールをファイル末尾に置くのは
Rust の慣習なので、この打ち切りで足りる。

🔴 **検出語は `concat!` で分割して書く。** 検査語をそのまま書くと、
検査器が自分自身を違反として報告する（実測: 初版で7件の自己検出が出た）。
ファイル単位の除外で黙らせると**そのファイルだけ他の CNF も効かなくなる**ので採らない。

### CNF-006 が閉じている輪

`#[expect]` の `reason` は `<規則 ID>: <理由>` の形でなければならず、
**その規則 ID が `docs/coding-rules.md` に実在すること**を検査する。

⇒ 規約から規則を消すと、それを引いていた抑制が CI で落ちる。
**規約とコードが片側だけ動く経路が無い。**

🔴 **依存を足さずに書くこと**（ARC-004）。CNF-0xx は全て構文で判定できる。
型情報が要る規則が出てきた時点で ADR を立てて再検討する。

## 7. 採用しなかった検査と、その理由

**「有効にしなかった」ことも決定である。** 再提案するときは、ここに書かれた理由への反論から始めること。

| 検査 | 不採用の理由 |
| --- | --- |
| `clippy::restriction` 群の一括有効化 | restriction は**互いに矛盾する規則の在庫**であって規則集ではない。実測で `mod_module_files` と `self_named_module_files` は正反対だった。群で入れると規則ではなく道具の機嫌に従うことになる |
| `clippy::nursery` 群の一括有効化 | 「まだ完成していない」と作者が宣言している lint 群である。誤検知を承知で入れると、抑制が常態化して二段構えの2段目が壊れる。**必要なものは1つずつ名指しで入れる**（`cognitive_complexity` はそうした） |
| `clippy::arithmetic_side_effects` | `a + 1` を全て `checked_add` に置き換えさせる。テキストのオフセット計算が主な仕事の道具では、ほぼ全行が対象になる。**代わりに `[profile.release] overflow-checks = true` を採った**（RS-017）。silent wrap という本当の危険は消え、記述の摩擦は残らない |
| `clippy::missing_docs_in_private_items` | 非公開の補助関数にまで doc を要求する。名前が役割を語っていれば（RS-013）重複であり、書かされた doc は更新されず嘘になる。公開項目の `missing_docs` は採用済み |
| `clippy::min_ident_chars` | `i` / `n` / `s` のような Rust で確立した短縮名と正面から衝突する。防いだ事故より生む摩擦のほうが大きい |
| `clippy::pattern_type_mismatch` | `&` の付け外しを全て明示させる。Rust の match ergonomics（言語が意図的に導入した省略）を丸ごと否定する取引になる |
| `clippy::unused_results` / `unused_results` | 戻り値のある式文すべてに `let _ =` を要求する。重要な取りこぼしは `#[must_use]` と `unused_must_use` が既に捕まえており、残りは雑音である |
| waiver 台帳（NENE-PIXEL の `WVR-NNNN`） | **Rust では制度の大半をコンパイラが執行している。** forbid 層は例外の申請窓口自体が存在せず、deny 層の `#[expect]` は理由が必須で、不要になれば失敗する。台帳は「機械が見ていない抑制」を人手で追うための仕組みであり、その対象がここには無い。**必要になったら（＝forbid を deny に降ろす判断をしたら）ADR で導入する** |
| `rustfmt.toml` | 整形の選択肢を作る。既定のまま使うことが「一つの手段」である |
| `cargo-deny` | 現在の依存が 0 なので、検査対象が存在しない。**依存を1つ足す ADR と同時に導入する**（ARC-004） |

---

## 8. 規約に違反したくなったとき

1. **まず、規約が間違っている可能性を検討する。** 規約は実装より新しくない
2. 規約が正しいなら、設計を変える。抑制で通さない
3. 抑制が要るなら `#[expect(<lint>, reason = "<規則 ID>: <理由>")]` を最小スコープに書く。
   **forbid 層は抑制できない**（E0453）。抑制したくなった時点で設計の問題である
4. **規則そのものを緩めるなら ADR を書く**（QLT-005）。
   旧規則のどの部分が今も有効かを明記すること
