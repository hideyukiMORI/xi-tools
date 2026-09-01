# ADR 0001: 厳格性は文章ではなく機械で強制する

## Status

accepted (2026-09-02)

## Context

施主から前提が示された（2026-09-02）:

> NENE2 の思想に倣ってガチガチに規約で固めたい。
> 一つの事を表現するためには必ず決められた一つの手段をとらざるを得ない状態を規約で固める思想。
> これに近いことを Rust で理想的な状態にできるか。

参照する先行事例が3つある。

| repo | 言語 | 形 |
| --- | --- | --- |
| `NENE2` | PHP | 散文の `coding-standards.md` ＋ PHPStan level 8 ＋ `composer check` ＋ 自作 `tools/conformance.php` |
| `NENE-PIXEL` | Kotlin | 規則 ID（`KOT-001`〜`KOT-022` / `ARC` / `QLT`）＋「機械強制: active / planned」表 ＋ baseline 禁止 ＋ **waiver 台帳（`WVR-NNNN`）** |
| `nene-recall` | Go | `NENE-PIXEL` の形式を継承し、`.golangci.yml` と自作 `tools/conformance` の3層に落とした |

**問題は、Kotlin で成立していたものが Go でそのまま成立しなかったことである。**
`nene-recall` の ADR 0010 が実測で記録している——Go には sum type が無く、
ゼロ値がコンストラクタを迂回し、未型付き定数が暗黙変換される。
`nene-recall` が達成したのは「不正な状態を表現不能にする」ではなく
「**不正な状態を書いたら CI が落ちる**」であり、両者は同じではない。

**したがって、Rust で同じ問いを立て直す必要がある。** Rust で実測した（実行環境: rustc 1.98.0 / edition 2024）。

```rust
mod org {
    pub struct Id(u64);                        // フィールドは非公開
    impl Id { pub fn new(v: u64) -> Option<Self> { ... } }
}

org::Id(1)             // ❌ E0603: tuple struct constructor `Id` is private
search(1)              // ❌ E0308: mismatched types（暗黙変換が無い）
let z: org::Id;
search(z)              // ❌ E0381: used binding `z` isn't initialized（ゼロ値が無い）

match m { Read => .., Write => .. }            // ❌ E0004: `Append` not covered
Config { addr: x, timeout: 1 }                 // ❌ E0063: missing field `retries`
```

`nene-recall` ADR 0010 が「Go に無い」と記録した4点は、**Rust では全てコンパイラが見ている。**

| NENE-PIXEL の規則 | Go | Rust（実測） |
| --- | --- | --- |
| KOT-002 不正状態を表現不能に・`else` なし網羅 | 無い | **ある**（`E0004`） |
| KOT-003 公開状態は不変 | 無い | **ある**（`let` が既定で不変） |
| KOT-004 `null` は一意の意味 | 不十分 | **ある**（`null` もゼロ値も無い） |
| KOT-007 private コンストラクタ＋唯一のファクトリ | 不十分 | **ある**（`E0603`） |

**そして、Rust には先行3事例のどれにも無かった機構がある。**

```rust
#![forbid(clippy::unwrap_used)]
#[expect(clippy::unwrap_used, reason = "テストだから")]   // ❌ E0453: incompatible with previous forbid
```

`forbid` 水準の lint は、**抑制しようとする行為そのものがコンパイルエラーになる。**
Kotlin の `@Suppress` にも Go の `//nolint` にも、これに相当するものは無い。
両者は「抑制に理由を要求する」までしかできず、だからこそ NENE-PIXEL は
人手の waiver 台帳を必要とした。

さらに測った:

```rust
#[expect(unused_variables, reason = "...")]   // 発火しない抑制
fn f() { let used = 1; println!("{used}"); }
// ❌ error: this lint expectation is unfulfilled
```

**発火しなくなった抑制が失敗する。** 抑制が腐ったまま残る経路が言語側に無い。

**そして、いま実装が空である。** `scopegrep` は `not implemented yet` を出して終わる足場だけで、
規則を後から被せる場合に必要になる baseline が、今は要らない。
違反ゼロから始められる時点は今しかない。

## Decision

**厳格性を「規約文書」ではなく「落ちる仕組み」として実装する。** 三層で強制する。

| 層 | 何を守るか | 実体 |
| --- | --- | --- |
| コンパイラ / cargo | 型・可視性・網羅性・クレート境界・依存の非循環 | Rust 本体、`Cargo.toml` の依存宣言 |
| lint | 書けてしまうが書くべきでないこと | `[workspace.lints]` ＋ `clippy.toml` |
| 規約検査 | xi-tools として守るべきこと | `xtask`（**未実装。全 CNF は planned**） |

具体的な決定は次のとおり。

1. **規則の正本を `docs/coding-rules.md` に置く。** すべての規則に ID
   （`RS-` / `ARC-` / `QLT-` / `CNF-`）を与え、機械強制の状態を
   **active / planned / 不能 / 不採用** で明示する。**planned を active と書かない。**

2. 🔴 **抑制を二段構えにする。これが本 ADR の中核である。**

   | 段 | 水準 | 抑制 | 対象 |
   | --- | --- | --- | --- |
   | 1段目 | `forbid` | **不可能**（E0453） | 例外を認めた瞬間に規則の意味が消えるもの |
   | 2段目 | `deny` | `#[expect(lint, reason)]` のみ | 「唯一の許可された場所」を宣言させたいもの |

   2段目は3つの lint で閉じる——`clippy::allow_attributes`(forbid) が `#[allow]` を封じ、
   `clippy::allow_attributes_without_reason`(forbid) が理由を必須にし、
   `unfulfilled_lint_expectations`(deny) が腐った抑制を落とす。

3. **`make check` を唯一の入口とする。** CI は `make check` を呼ぶだけにし、
   CI 側にしか無い検査を作らない。「手元で再現できない失敗」を構造的に消すため。

4. **baseline を持たない。** クレート単位の `#![allow]` は
   `clippy::allow_attributes` が forbid なので**そもそも書けない**。

5. **ゲートを弱める変更は ADR を要する。** 閾値・`forbid` からの降格・lint の削除・
   免除の追加が対象。CI は該当ファイルの差分を PR に通知する。

6. **道具の版は `rust-toolchain.toml` だけが決める。** `Makefile` にも CI にも版を書かない。
   2箇所に書くと、片方だけ上げられて「手元では通る」が生まれる。

7. **ゲートは意図的な違反で発火することを証明してから active と書く。**
   手順と実出力は `docs/quality/gate-proofs.md`。

8. **依存はゼロのまま始める。** `cargo-deny` も導入しない（検査対象が無い）。
   依存を1つ足す ADR と同時に導入する。

### 却下した選択肢

| 選択肢 | 却下の理由 |
| --- | --- |
| `clippy::restriction` 群の一括有効化 | restriction は**互いに矛盾する規則の在庫**である。実測で `mod_module_files` と `self_named_module_files` は正反対だった（一方は `mod.rs` を禁じ、他方は要求する）。群で入れると規則ではなく道具の機嫌に従うことになる |
| `clippy::nursery` 群の一括有効化 | 作者が「まだ完成していない」と宣言している群。誤検知を承知で入れると抑制が常態化し、二段構えの2段目が壊れる。必要なものは1つずつ名指しで入れる |
| 既定の clippy（`correctness` のみ）で始める | 規則と lint の対応が付かず、「なぜ有効か」を説明できない。厳格さが道具の既定値に委ねられる |
| **NENE-PIXEL の waiver 台帳（`WVR-NNNN`）を移植する** | **Rust では制度の大半をコンパイラが執行している。** forbid 層は例外の申請窓口が存在せず、deny 層の `#[expect]` は理由が必須で、不要になれば自動で失敗する。台帳は「機械が見ていない抑制を人手で追う」ための仕組みであり、その対象がここには無い。**移植すると、機械が既に守っているものを人手で二重に追う形になる**。必要になったら（＝forbid を deny に降ろす判断をしたら）ADR で導入する |
| `clippy::arithmetic_side_effects` を有効化する | `a + 1` を全て `checked_add` に置き換えさせる。テキストのオフセット計算が主な仕事の道具ではほぼ全行が対象になる。代わりに `[profile.release] overflow-checks = true` を採った。**silent wrap という本当の危険は消え、記述の摩擦は残らない** |
| 規約を `CLAUDE.md` にだけ書く | 人間と AI の遵守に依存する。`nene-recall` で実証済みの失敗——`org_id` の規約は CLAUDE.md にあったが、守っていたのはテスト10ケースだけで、新しく書かれるコードには及んでいなかった |
| 実装が入ってから導入する | 既存違反を凍結する baseline が必要になる。今なら違反ゼロで始められる |
| `rustfmt.toml` を置く | 整形の選択肢を作る。既定のまま使うことが「一つの手段」である |

## Consequences

**得るもの**

- 規則が「読まれなければ効かないもの」から「書いた瞬間に落ちるもの」になる。
  次に書くのが人間でも AI でも、遵守が意思に依存しない
- **抑制の腐敗が構造的に起きない。** 不要になった `#[expect]` は CI が落とす
- **waiver 台帳という人手の制度を1つ減らせた。** 先行事例より規約が軽く、かつ強い
- 依存が 0 なので、供給網の risk とライセンスの検討が現時点で不要

**払うもの**

- 書き味に制約が入る。`as_conversions`（`as` 禁止）・`default_numeric_fallback`
  （数値リテラルの型注釈必須）・`indexing_slicing`（`v[i]` 禁止）は、いずれも
  「短く書く」ことを妨げる。特に `scopegrep` は**オフセット計算が主な仕事**なので、
  実装フェーズで最初に摩擦が出るのはここだと予想する
- `forbid` は後から緩めにくい。降格には ADR が要る（それが狙いだが、コストである）
- clippy の lint 名は版によって削除・改名される（実測: `clippy::string_to_string` は
  1.98 で削除されていた）。toolchain を上げるときは lint の生存確認が要る

**正直に記録しておくこと**

- 🔴 **「不正な状態を表現不能にする」は Rust でも完全ではない。**
  `#[derive(Default)]` を書けば不変条件を持つ型のゼロ値を自分で作れる。
  また Rust の可視性は「子モジュールが祖先の非公開項目に到達できる」ので、
  同じモジュールに同居する型はファクトリを迂回できる。
  この2つは `CNF-001` / `CNF-005` で塞ぐ予定だが、**現時点では planned であり、
  規約文書の言葉としてしか存在しない**
- 🔴 **`clippy.toml` に閾値を書いても、lint 本体が無効なら何も起きない。**
  本 ADR の作業中に `cognitive_complexity` が実際にこの状態だった
  （nursery 群にあり、どの既定群にも入らない）。**設定が在ることは、検査が効いていることの証明ではない。**
  QLT-007 の証明手順が無ければ、これは緑のまま気づかれなかった
- **本 ADR の厳格さは、実装がまだ空である今だからこそ無傷で導入できた。**
  緑であることは、規則が良いことの証明ではない。検査対象がまだ小さいことの結果でもある。
  実装で規則が邪魔になったとき、緩めるのではなく **ADR で判断を残すこと**が本 ADR の眼目である

**追随作業**

- `CNF-001`〜`CNF-006` を `xtask` として実装する。**着手前に planned を active と書き換えないこと**
- `scopegrep` の中核クレートを切る時点で `ARC-003`（`no_std` による純粋性）が active になる
- 実装が入った時点で `QLT-008`（カバレッジ下限）を `Makefile` に置く

## Related

- Issue: なし
- PR: なし
- Supersedes: none
- Superseded by: none
- 関連: `nene-recall` ADR 0010（本 ADR が Rust 向けに問い直したもの）、
  `NENE-PIXEL` `docs/CODING_RULES.md` / `docs/QUALITY_GATES.md`（規則 ID と機械強制表の形式の出所）
