# リリース手順（内部）

`scopegrep` を crates.io と GitHub Releases に出すときの手順。**順番に意味がある**ので、
気持ちで前後させない。判断の背景は [`../CLAUDE.md`](../CLAUDE.md)、規約は
[`coding-rules.md`](./coding-rules.md)。

🔴 **この文書は施主が手で実行する手順である。** 自動化していないのは、
publish が取り消せない操作（crates.io は削除ではなく yank しかできない）だからで、
「間違えても戻せる」形になるまでは人が引き金を引く。

---

## 0. 前提

- `main` が緑（CI）
- 作業ツリーがクリーン。`cargo package` は汚れたツリーを拒む。
  🔴 **`--allow-dirty` で黙らせないこと。** 何を配ったかが git から辿れなくなる
- `cargo login` 済み（crates.io の API トークン）

## 1. 版を上げる

版の正本は `Cargo.toml` である。**3箇所を同時に直す**（1つ忘れると 4 で落ちる）。

| 場所 | |
| --- | --- |
| `scopegrep-core/Cargo.toml` | `version` |
| `scopegrep/Cargo.toml` | `version` |
| `scopegrep/Cargo.toml` | 依存宣言 `scopegrep-core = { path = "...", version = "..." }` |

`Cargo.lock` を追随させる（`cargo metadata --format-version 1 >/dev/null` でよい）。
`--locked` を使う検査が全て落ちるので、忘れても必ず気づく。

## 2. CHANGELOG を書く

[`../CHANGELOG.md`](../CHANGELOG.md) に新しい版の節を足す。書くのは**この版で実際に変わったこと**だけ。
「速い」「使いやすい」のような、測っていない形容を書かない（QLT-009）。

## 3. 検査

```bash
make check
make check-version TAG=v0.1.0
make package
```

- `make check` — 提出前に必ず通すもの。CI と同じ
- `make check-version` — タグと `Cargo.toml` の版の一致。**Release workflow が同じものを呼ぶ**ので、
  ここで通ればタグを打った後に落ちない
- `make package` — `.crate` を実際に作る。🔴 **2つを1回の `cargo package` で作る**。
  分けて打つと、まだ core が crates.io に無い初回は
  `no matching package named scopegrep-core found` で落ちる（`Makefile` のコメントに実測を記録）

## 4. PR を出してマージする

版と CHANGELOG の変更は PR にする。**publish は必ずマージ後**。
crates.io に出したものは消せないので、レビューを通っていないものを先に出さない。

## 5. publish（順番を変えられない）

マージ後の `main` を pull し、クリーンな状態で:

```bash
cargo publish -p scopegrep-core
#   ここで数分待つ（index に出るまで bin 側の解決が失敗する）
cargo publish -p scopegrep
```

🔴 **core より先に bin を publish できない。** `scopegrep` の依存宣言は
`{ path = "../scopegrep-core", version = "x.y.z" }` で、publish されるのは `version` の側である。
crates.io に該当版の `scopegrep-core` が無い間、`cargo publish -p scopegrep` は
`no matching package named scopegrep-core found` で落ちる。

⚠️ **待ち時間は「index に反映されるまで」であって、決まった秒数ではない。**
落ちたら数分置いてもう一度打つ。焦って `--no-verify` を付けない
（付けても、解決に失敗する原因は消えない）。

## 6. タグを打つ

```bash
git tag v0.1.0
git push origin v0.1.0
```

🔴 **タグは publish の後**。タグを push した瞬間に
[`../.github/workflows/release.yml`](../.github/workflows/release.yml) が走り、
GitHub Releases に binary が並ぶ。crates.io 側が失敗したままリリースだけが出る状態を作らない。

## 7. Release workflow が走る

| job | すること |
| --- | --- |
| `verify` | `make check-version TAG=<タグ>` → `make check`。**タグと版が食い違ったらここで止まる** |
| `build` | Linux x86_64 / macOS arm64 / Windows x86_64 の binary（`--features regex`）と `sha256` |
| `release` | `gh release create <タグ> --generate-notes` に成果物を添付 |

⚠️ **書き込み権限を持つのは `release` job だけ**である。job を足すときに
`permissions: contents: write` を上位に移さないこと。

## 8. 失敗したときに戻れる範囲

| | 戻せるか |
| --- | --- |
| crates.io の publish | **戻せない**。`cargo yank` で新規の依存を止められるだけで、消えはしない |
| GitHub Release | 消せる（`gh release delete <タグ>`） |
| タグ | 消せる（`git push origin --delete <タグ>`）。ただし publish 済みの版を打ち直さない |

⇒ **取り返しがつかないのは 5 だけである。** 3 と 4 を飛ばさないことが、この手順の全部である。
