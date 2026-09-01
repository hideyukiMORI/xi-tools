# ゲート発火の証明

> 2026-09-02 実施 / rustc 1.98.0 (88d9e12ae) / clippy 0.1.98

🔴 **検査の最大の失敗は、見逃したまま常に緑を返すことである。**
本物のコードを見ている限り、それは永久に発覚しない。
したがって `docs/coding-rules.md` で **active** と書く前に、
**意図的な違反でゲートが落ちること**を確認し、ここに実出力を残す。

## 手順

1. `scopegrep/src/main.rs` を退避する
2. 違反コードを追記する
3. `cargo clippy --workspace --all-targets --locked -- -D warnings`（または該当ターゲット）を走らせる
4. 退避したファイルを戻し、`make check` が緑に戻ることを確認する

## 結果

| # | 意図的な違反 | 守る規則 | 実際の出力 |
| --- | --- | --- | --- |
| P-1 | forbid した lint を `#[expect]` で抜ける | QLT-006 | `error[E0453]: expect(clippy::unwrap_used) incompatible with previous forbid` |
| P-2 | 発火しない `#[expect]` を残す | QLT-006 | `error: this lint expectation is unfulfilled` |
| P-3 | `#[allow]` を書く | QLT-006 | `error: #[allow] attribute found` / `error: 'allow' attribute without specifying a reason` |
| P-4 | `match` に `_` を書く | RS-002 | `error: wildcard matches only a single variant and will also match any future added variants` |
| P-5 | `as` で数値変換する | RS-007 | `error: casting 'u64' to 'u32' may truncate the value` / `error: using a potentially dangerous silent 'as' conversion` |
| P-6 | `HashMap` を使う | RS-016 | `error: use of a disallowed type 'std::collections::HashMap'` |
| P-7 | 型注釈の無い数値リテラル | RS-018 | `error: default numeric fallback might occur` |
| P-8 | 未文書の公開項目 | RS-008 | `error: missing documentation for a function` |
| P-9 | `mod.rs` を作る | RS-012 | ``error: `mod.rs` files are not allowed, found `scopegrep/src/probe9/mod.rs` `` |
| P-10 | rustfmt 差分を残す | QLT-004 | `Diff in .../scopegrep/src/main.rs:33` |
| P-11 | 認知的複雑度 25 の関数 | RS-011 | `error: the function has a cognitive complexity of (25/10)` |

**復旧確認**: 全ケースでファイルを戻したあと `make check` が緑に戻ることを確認済み。

## 言語側の証明（lint ではなくコンパイラが守るもの）

| # | 意図的な違反 | 守る規則 | 実際の出力 |
| --- | --- | --- | --- |
| C-1 | 非公開フィールドの newtype をモジュール外で直接構築 | RS-001 | `error[E0603]: tuple struct constructor 'Id' is private` |
| C-2 | 未型付きリテラルを newtype の引数に渡す | RS-001 | `error[E0308]: mismatched types` |
| C-3 | 未初期化の束縛を使う（Go の「ゼロ値」に相当） | RS-003 | `error[E0381]: used binding 'z' isn't initialized` |
| C-4 | enum の variant を1つ落とした `match` | RS-002 | `error[E0004]: non-exhaustive patterns: 'Mode::Append' not covered` |
| C-5 | 構造体の部分構築 | RS-003 | `error[E0063]: missing field 'retries' in initializer of 'Config'` |
| C-6 | `#![forbid(unsafe_code)]` を `#[allow]` で抜ける | RS-010 | `error[E0453]: allow(unsafe_code) incompatible with previous forbid` |
| C-7 | `no_std` クレートから `std::time` を使う | ARC-003 | `error[E0433]: cannot find module or crate 'std' in this scope` |
| C-8 | クレート間の循環依存 | ARC-002 | ``error: cyclic package dependency: package `a` depends on itself`` |

## 規約検査（CNF-0xx）の証明

`xtask` の検査は **`cargo test -p xtask` の 22 ケース**が、
「意図的な違反で発火すること」と「正しいコードでは発火しないこと」の両方を持つ。
検査を足すときは、この両方を同じコミットで書くこと。

| 検査 | 発火の証明 | 空振りしないことの証明 |
| --- | --- | --- |
| `CNF-001` | derive / impl / `..Default::default()` の3形 | 正しいファクトリ・テストモジュール内・`.md` は対象外 |
| `CNF-002` | 遅延グローバル・proc-macro マニフェスト | 素の関数 |
| `CNF-003` | 2つ目の型宣言 | 単独宣言・関数内のローカル型 |
| `CNF-004` | 禁止語尾・禁止モジュール名 | **`Processor` は通る**（判断が要る語を機械で拒否しないことの証明） |
| `CNF-006a` | 実在しない規則 ID・規則 ID の無い reason・複数行 `#[expect]` | 実在する規則 ID |
| `CNF-006b` | 壊れた相対リンク | 外部 URL・アンカー |

### リポジトリの実ファイルでの発火

単体テストだけでは「検査器が repo に繋がっているか」を証明できないので、実ファイルでも確認した。

| # | 意図的な違反 | 実際の出力 |
| --- | --- | --- |
| P-12 | `docs/todo/current.md` のリンク先を実在しないファイルにする | `CNF-006: docs/todo/current.md:25 — リンク先 '../../scopegrep/testdata/nonexistent.yml' が存在しない` |
| P-13 | `main.rs` の `#[expect]` の reason を `RS-999` に書き換える | `CNF-006: scopegrep/src/main.rs:15 — 'RS-999' は docs/coding-rules.md に無い規則 ID である` |

## この手順で見つかった実際の欠陥

🔴 **`cognitive_complexity` が「閾値だけ設定され、lint は無効」という死んだ状態だった。**

`clippy.toml` に `cognitive-complexity-threshold = 10` を書いていたが、
この lint は `nursery` 群にあり、`clippy::all` にも `pedantic` にも含まれない。
`Cargo.toml` 側で明示的に有効化していなかったため、**閾値は一度も評価されていなかった。**

認知的複雑度 25 の関数を書いても何も起きないことを P-11 の手順で確認し、
`Cargo.toml` に `cognitive_complexity = "deny"` を追加して再測定した。

🔑 **設定ファイルに項目が在ることは、検査が効いていることの証明ではない。**
本物のコードだけを見ていたら、この穴は永久に緑のままだった。

### 検査器が自分自身を検出した（CNF-002 / CNF-001）

`xtask` を初めてリポジトリに走らせたところ、**違反7件を報告した。全て検査器自身の
検出語**だった——`FORBIDDEN_CONSTRUCTS` に並べた `dyn Any` などの文字列と、
違反メッセージに含めた `..Default::default()` である。

ファイル単位の除外で黙らせると、**そのファイルだけ他の CNF も効かなくなる**。
検出語を `concat!` で分割して書く形に変えた（`xtask/src/check.rs` の該当箇所に理由を併記）。

🔑 **検査器は自分自身も検査対象である。** 除外で逃げると、
「検査器のコードだけ規約の外にある」という穴が静かに開く。

## 追随

- `CNF-001`〜`CNF-006`（`xtask`）を実装したら、各検査について
  「意図的な違反で発火すること」のテストを**検査器と同じコミットで**書き、ここに追記する
- toolchain を上げたときは、削除・改名された lint が無いか確認する
  （実測: `clippy::string_to_string` は 1.98 で削除済みだった）
