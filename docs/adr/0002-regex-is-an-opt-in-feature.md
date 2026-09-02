# ADR 0002: 正規表現は opt-in の feature で足し、既定の依存は 0 のまま保つ

## Status

accepted (2026-09-02・施主承認)

## Context

`scopegrep` の検索は固定文字列だけである（設計メモ「検索の意味」）。施主から「便利に使えるレベル」への
要求があり、`grep` の代替として使うなら正規表現が要る場面が出る。

一方で本リポジトリは **依存 0** を前提に置いている（ARC-004。単一バイナリで配れること、
`scopegrep-core` が `no_std` で純粋であること）。`regex` crate を足すと推移的に 5 crate
（`regex` / `regex-syntax` / `regex-automata` / `aho-corasick` / `memchr`）が入る。

ARC-004 は「依存を足すときは ADR」「`cargo-deny` は依存を 1 つ足す ADR と同時に導入する」と定めている。
本 ADR がその最初の 1 本である。

施主の提案は「**オプションで拡張できる形**」（2026-09-02）。Cargo の feature で切る。

## Decision

1. **`scopegrep` に Cargo feature `regex`（既定 off）を設ける。** `cargo install --path scopegrep` は今までどおり依存 0。
   `--features regex` を付けたときだけ `regex` crate が入り、CLI の `-e` / `--regex` が使える
2. **`scopegrep-core` は `no_std`・依存 0 のまま変えない。** 一致の判定は core が `Matcher` の trait で受け取り、
   固定文字列の実装は core に、正規表現の実装は `scopegrep`（bin）側に `#[cfg(feature = "regex")]` で置く。
   `#[cfg]` の分岐は bin 側だけに閉じる
3. **feature 無しでビルドした binary で `--regex` を打つと、終了 2 で「この binary は正規表現なしでビルドされている」と言う。**
   黙って固定文字列として扱わない
4. **`cargo-deny` を同時に導入する。** `deny.toml` を置き、`make check` に `deny` を足す。
   ライセンスは許可制（MIT / Apache-2.0 / Unicode-3.0 / BSD 系を許可・GPL 系は拒否）、advisories は警告ではなく失敗、
   重複バージョンは失敗
5. **`make check` は feature あり・なしの両方でテストを回す。** 片方だけ緑の状態を作らない

### 却下した選択肢

| 選択肢 | 却下の理由 |
| --- | --- |
| 正規表現を常時有効にする | 既定の依存が 0 でなくなる。単一バイナリで配れることと `no_std` の中核は残せるが、「依存 0」という README の主張が消える。要らない人にも 5 crate を配る |
| 正規表現エンジンを自分で書く | 道具の目的（所属を返す）から外れた場所に工数と欠陥を積む。「Rust でツールが作れる」を示す第一目的に対して、正規表現エンジンの自作は寄与より欠陥のリスクが大きい |
| グロブ（`*` / `?`）だけ足す | 固定文字列と正規表現の間に三つ目の記法を作る。「一つの事に一つの手段」に反する。`--scope` のセグメント一致とも混同を招く |
| `scopegrep-regex` を別クレートにする | クレートが 1 つ増える分の管理（publish・版）に対して、得るものは feature と同じ。ARC-001（1 ツール = 1 クレート）の趣旨にも合わない |
| `regex-lite`（依存が少ない代替） | 依存は減るが機能と性能が落ち、`regex` との**二択が生まれる**。採るなら 1 つ。`regex` の推移的依存 5 件は `cargo-deny` で見える範囲に収まる |

## Consequences

**得るもの**

- 既定の依存 0 と `no_std` の中核を保ったまま、必要な人だけが正規表現を使える
- `cargo-deny` が入り、ARC-004 の planned（ライセンス・脆弱性・重複バージョンの検査）が active になる
- 「依存 0 の中核を守りながら opt-in で拡張する」構造を実装で示せる（第一目的）

**払うもの**

- `Cargo.lock` には `regex` 系が常に入る。「依存 0」は「**既定ビルドの依存 0**」と言い換える。README でもそう書く
- `make check` のテスト時間が feature 分だけ増える（数十秒）
- `#[cfg(feature)]` の分岐が bin 側に入る。テストも両構成で通す必要がある

**正直に記録しておくこと**

- `regex` の一致は**行単位**（`^` `$` は行の先頭と末尾。複数行スカラーを跨ぐ一致はしない）。
  値は行ごとに持つ設計（設計メモ）なので、これは制約ではなく既定の帰結だが、README に書く
- `-i` は固定文字列では文字ごとの case fold、正規表現では `(?i)` 相当。**同じ旗で意味が微妙に違う**
  （Unicode の扱いが `regex` の実装に依存する）。差が問題になったら ADR を切る

**追随作業**

- `deny.toml` と `make deny`（本 ADR と同じ PR）
- `docs/coding-rules.md` ARC-004 の planned → active（`cargo-deny` 導入後・発火の証明を `gate-proofs.md` に）
- README の「依存は 0」を「既定ビルドの依存は 0」に

## Related

- Issue: none
- PR: `#4`（予定）
- Supersedes: none
- Superseded by: none
