# scopegrep 設計メモ

**状態: 設計確定（2026-09-02）。実装中。** D-2 の実測値は本文末尾の「D-2 実測」節。

## 解く問題

行ベースの検索は「その行がある」ことしか返さない。構造を持つ設定ファイル
（YAML / TOML / JSON）では、**知りたいのは行番号ではなく所属**である。

実害の記録（2026-09-01）:

1. `frontend-ci.yml` を `cancelled()` で検索し、ヒットした行番号の並びから
   「2リポに同じ欠陥がある」と判断した。**片方は別ジョブの正しい用法だった**
2. 同じ検索がファイル名を `frontend-ci.yml` に限定していたため、
   **同じ欠陥を持つ `check.yml` を見落とした**
3. 所属を出すよう `awk` で作り直したが、**コメント内の一致を拾う偽陽性が残った**

⇒ **1回の粗い検索が、偽陽性と偽陰性を同時に生む。** 片方だけ疑っても見つからない。

## 決定（D-1〜D-4）

| | 論点 | 決定 |
| --- | --- | --- |
| D-1 | 所属の表現 | **両方持つ。** 人向けは `jobs.e2e.steps[2] "Upload Playwright report" .if`、機械向けは RFC 6901 の JSON Pointer `/jobs/e2e/steps/2/if` |
| D-2 | パーサ | **手書きの行指向スキャナ（依存 0・`no_std`）。** 対応する YAML の部分集合を明示し、**部分集合の外は黙って誤読せずエラーにする** |
| D-3 | `name:` の無いステップ | **索引だけ出す。** 補助表示（`run:` の先頭など）はしない |
| D-4 | 出力形式 | **人向け1行が既定、`--json` で JSON Lines。** 両方とも順序が決定的 |

### D-1 — 所属の表現は両方持つ

人向けの形は README が最初から約束していた形である。機械向けは JSON Pointer を採る。
**既存の規格を使えば、`jq` や他ツールとの接続を自分で設計しなくてよい。**

| 却下した案 | 理由 |
| --- | --- |
| 人向けの形だけ | ラベル（`"Upload Playwright report"`）を含む文字列は機械が分解しにくい。パイプの先で使えない |
| JSON Pointer だけ | `steps/2` を見ても、それがどのステップか分からない。**最初の実害はまさに「行番号では分からない」ことだった** |
| 独自のパス記法（`jobs->e2e->steps#2`） | 既に規格がある物に二つ目の記法を作らない |

**ラベルの規則**: シーケンスの要素がマッピングで、`name` キーの値が1行のスカラーなら、
`[i]` の直後に ` "<name>"` を付ける。これは GitHub Actions / Ansible / Kubernetes に共通する
慣習であって YAML の規格ではない。だから **JSON には `label` として独立したフィールドで出し、
`pointer` には混ぜない**。ラベルが無ければ `[i]` だけになる（D-3）。

ラベルはキーと同じく**クォートを1枚外した**テキスト（`name: "Build"` のラベルは `Build`。エスケープは解除しない）。
ラベルは表示のための識別子であって検索対象ではないので、値の「原文のまま」規則は適用しない。
`name:` が他のキーより後に書かれていてもラベルは付く（走査後に当てはめる）。

### D-2 — 手書きの行指向スキャナ

選定基準は「**コメント内のヒットを設定値と区別できる**こと」が第一で、
規約から来る制約（依存 0・`build.rs` 禁止・`no_std`）が第二である。
実測値は末尾の「D-2 実測」節。**判断の要点:**

- この道具が読むのは CI 設定・compose・manifest のような**人が手で書く YAML**であり、
  YAML 1.2 の全機能ではない。**必要なのはコメントと値の区別、行番号、入れ子であって、
  それは行ごとのインデントから復元できる**
- 部分集合の外（アンカー・複数行のフロー記法など）は**エラーとして報告する**。
  黙って誤読した結果を返すよりも、「このファイルは読めない」と言うほうが道具として誠実である
- 「Rust でツールが作れる」ことを示すのが第一目的（CLAUDE.md）である。
  既存パーサを包むだけの実装より、**スキャナを書いてテストで守る**ほうがその目的に合う

| 却下した案 | 理由 |
| --- | --- |
| `serde_yaml` / `serde_yml` で値だけ読む | スカラーの位置情報が取れない。行番号を返せない道具に存在理由が無い |
| イベント/位置付きのパーサ crate（`saphyr-parser` / `yaml-rust2` / `marked-yaml`） | 位置は取れ、fixture も通る（実測）。**しかしコメントを捨てる**ので「コメント内のヒット」を別枠で見せる拡張ができない。依存が 7〜9 件増え（`saphyr-parser` は `thiserror` 経由で proc-macro 連鎖を抱える）、ADR と `cargo-deny` の導入を要する（ARC-004）。`no_std` で通ったのは `saphyr-parser` だけ（実測）。手書きが fixture を通せない場合の**次点は `saphyr-parser`** |
| `tree-sitter-yaml` / `noyalib` | コメントも位置も持ち、**唯一コメント内ヒットを別枠で出せた**（実測）。しかし `tree-sitter` は **`build.rs` で C をコンパイルし C コンパイラが必須**（RS-010 に抵触・ADR が要る）、`noyalib` は依存 11 件で最多かつ pre-1.0 で版が速く動く。手書きスキャナはコメントの位置を自分で知っているので、同じ拡張を依存なしで後から足せる |

🔴 **この決定は「規約が設計を殺した」形にしない。** 手書きスキャナが fixture を通せず、
かつ部分集合を広げても解けない構造が出た時点で、次点（位置付きパーサ crate）へ ADR 付きで移る。

### D-3 — ラベルが無ければ索引だけ

`run:` の先頭や `uses:` の値を代わりに見せる案は、**どのキーを見せるかがファイルの種類ごとに変わる**。
判断を道具に持ち込むと出力の意味が「文脈次第」になる。索引だけなら意味は一つである。

### D-4 — 人向け1行が既定、`--json` で JSON Lines

- 人向け: `grep -n` と同じ `<file>:<line>:` で始める。エディタや `grep` 前提の道具がそのまま拾える
- 機械向け: 1ヒット1行の JSON。**ストリームで処理できて、途中で切れても壊れない**
- `--show-scope` フラグは**設けない**。所属を出すことがこの道具の存在理由であり、出さないモードは要らない

---

## アーキテクチャ

```
scopegrep-core/     #![no_std] + alloc。依存 0。読む・組み立てる・探す。I/O を持たない
scopegrep/          bin。引数・ファイル走査・出力。std に触るのはここだけ
```

| 規則 | ここでの実体 |
| --- | --- |
| ARC-002 層はクレート | `scopegrep` → `scopegrep-core` の一方向。逆は cargo が拒む |
| ARC-003 中核は `no_std` | `scopegrep-core` に `#![no_std]`。`std::fs` / `std::env` は**名前解決エラー** |
| RS-015 環境に触るのは配線点 | `scopegrep/src/main.rs` だけが `std::env::args` を読む |
| RS-014 出力は1箇所 | `scopegrep/src/output.rs` だけが `print!` / `eprint!` を書く |

### `scopegrep-core` の公開 API

```rust
/// YAML を読んで構造を組み立てる。部分集合の外は `ParseError`。
pub fn parse(source: &str) -> Result<Document, ParseError>;

impl Document {
    /// 固定文字列 `needle` を含むスカラー値を、出現順（行・列）で返す。
    pub fn search(&self, needle: &str) -> Vec<Hit>;
}

impl Hit {
    pub fn path(&self) -> &ScopePath;
    pub fn line(&self) -> LineNumber;       // 1 始まり
    pub fn column(&self) -> Column;         // 一致の先頭。1 始まり・文字数（バイトではない）
    pub fn value(&self) -> &str;            // ヒットした行のスカラーテキスト（原文のまま）
}

impl ScopePath {
    pub fn pointer(&self) -> String;        // "/jobs/e2e/steps/2/if"（RFC 6901。`~`→`~0` `/`→`~1`）
    pub fn label(&self) -> Option<&str>;    // 最も内側のシーケンス要素の `name`
}
impl core::fmt::Display for ScopePath      // 人向け: jobs.e2e.steps[2] "Upload Playwright report" .if
```

- `LineNumber` / `Column` は非公開フィールドの newtype（RS-001）。0 は作れない
- 型ごとに1ファイル（RS-012 / CNF-003）。`ParseError` は `line()` と `Display` を持ち、`core::error::Error` を実装する
- **`Document` の内部表現（木か平坦な表か）は公開しない。** API は上の4つの型だけ
- `pub use` を書かない（RS-008）ので、型はモジュール経由で見える（`scopegrep_core::document::Document` 等）。crate 直下にあるのは `parse` だけ

### 検索の意味

- `needle` は**固定文字列**（正規表現ではない）。大文字小文字を区別する。
  正規表現は `regex` crate を要し、ARC-004 の ADR になる。**要るようになったら足す**
- 探すのは**スカラー値だけ**。キーとコメントは探さない
- 1つのスカラー行に needle が複数回あっても**ヒットは1つ**（`grep` と同じ行単位）。列は最初の出現位置
- 値は**原文のまま**照合する（クォートの中身をエスケープ解除しない）。
  人が `grep` で見る文字列と同じものに当たることを優先する
- 順序は「ファイル → 行 → 列」で決定的（RS-016）

---

## 対応する YAML の部分集合（v1）

🔴 **ここに書いていない構文はエラーである。黙って誤読しない。**
エラーはファイル単位で報告し、他のファイルの処理は続ける。

### 読めるもの

| 構文 | 扱い |
| --- | --- |
| コメント `# ...` | 行頭（空白のみの後）の `#`、または**空白の直後**の `#` から行末まで。**クォートの中の `#` はコメントではない** |
| ブロックマッピング `key: value` / `key:` + 入れ子 | キーはプレーン・`"…"`・`'…'`。`:` の後は空白か行末 |
| ブロックシーケンス `- item` | `- key: v` で始まるマッピング（後続キーは最初のキーの桁に揃う）、`-` の後の入れ子、スカラー |
| マッピング直下の同じ桁のシーケンス | `steps:` の次の行が同じ桁の `- ` で始まる形（YAML が許す省略） |
| スカラー（1行） | プレーン・`'…'`（`''` エスケープ）・`"…"`（`\` エスケープ）。**値は原文のまま持つ** |
| ブロックスカラー `\|` / `>` | チョンピング `+` `-`、インデント指示子（1桁）。内容の各行を**1行ずつ別のスカラー行**として持つ。**内容の中の `#` はコメントではない** |
| 1行のフロー記法 `[a, b]` / `{a: 1}` | **1つのスカラーとして原文のまま持つ**（中に入らない）。括弧が行内で閉じないときはエラー |
| 空の値 `key:` | null。検索対象にならない |
| 先頭の `---` | 読み飛ばす。`%YAML` 等のディレクティブはエラー |
| `\r\n` / BOM | 取り除く |

### エラーにするもの（v1 の外）

| 構文 | エラー種別 |
| --- | --- |
| アンカー `&a`・エイリアス `*a`・タグ `!!str` `!x`・マージキー `<<:` | `Unsupported` |
| 複数行のプレーンスカラー（継続行）・複数行のクォート | `Unsupported` |
| 複数行にまたがるフロー記法 | `Unsupported` |
| 2つ目の `---`・`...`（複数ドキュメント） | `Unsupported` |
| 複合キー `? ` | `Unsupported` |
| タブによるインデント | `Malformed` |
| インデントの矛盾（親より浅い位置に子が来る等） | `Malformed` |

`ParseError` は**種別と行番号**を持つ。「何行目の何が読めなかったか」を必ず言う。

---

## CLI（`scopegrep` バイナリ）

```
scopegrep [--json] <needle> <path>...
scopegrep --help | --version
```

| 引数 | 意味 |
| --- | --- |
| `<needle>` | 固定文字列 |
| `<path>` | ファイルなら拡張子を問わず読む。ディレクトリなら再帰して `.yml` / `.yaml` だけ読む。走査順はパスのバイト順で決定的。`.git` ディレクトリだけ飛ばす。シンボリックリンクは辿らない |
| `--json` | JSON Lines で出す |
| `--` | 以降を引数として扱う |

**引数の解析は手書き**（依存 0）。`clap` は ARC-004 の ADR になる。**要るようになったら足す**。

### 終了コード（`grep` と同じ）

| コード | 意味 |
| --- | --- |
| 0 | 1件以上ヒット |
| 1 | ヒット無し |
| 2 | エラーがあった（使い方・読めないファイル・部分集合の外）。**ヒットがあっても 2** |

### 出力

**人向け（既定）** — 1ヒット1行、標準出力:

```
<file>:<line>: <path> = <value>
```

```console
$ scopegrep 'cancelled()' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:33: jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .if = ${{ !cancelled() }}
scopegrep-core/testdata/workflow-with-comment.yml:46: jobs.e2e.steps[2] "Upload Playwright report" .if = ${{ !cancelled() }}
```

- `<path>` の人向け形式: キーは `.` で繋ぐ。索引は `[i]`（0 始まり）。ラベルがあれば `[i] "label"` の後に ` .key` と続ける。
  キーに `[A-Za-z0-9_-]` 以外の文字が含まれるときは `"…"` で囲む。ラベルの中の `"` と `\` は `\` でエスケープする
- `<value>` はヒットした行のスカラーテキスト（原文）。ブロックスカラーなら**その行**だけ

**機械向け（`--json`）** — 1ヒット1行、キーはこの順で固定:

```json
{"file":"scopegrep-core/testdata/workflow-with-comment.yml","line":33,"column":18,"pointer":"/jobs/frontend-check/steps/3/if","path":"jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if","label":"Audit (fail on high/critical)","value":"${{ !cancelled() }}"}
```

- `label` が無いときは `null`。キーは常に7つ
- JSON のエスケープは RFC 8259（`"` `\` と制御文字）。手書きで足りる。`serde_json` は ADR になる

**エラー** — 標準エラー、1件1行:

```
scopegrep: <file>:<line>: <message>
scopegrep: <file>: <io message>
scopegrep: usage: ...
```

---

## テスト（何が壊れたら分かるか）

| 層 | テスト |
| --- | --- |
| `scopegrep-core` | 部分集合の各構文を1つずつ通す単体テスト。**エラーにする構文が実際にエラーになるテスト**。fixture `testdata/workflow-with-comment.yml` で3つの `cancelled()` のうちコメント内だけが落ちることの検証 |
| `scopegrep` | バイナリを実際に起動する統合テスト。標準出力の**完全一致**、終了コード 0 / 1 / 2 |
| README | README の `$ scopegrep …` の出力例が**実際の出力と一致する**テスト（地雷2 を機械で塞ぐ） |

---

## 非目標

- YAML の値を書き換えない。**読むだけ**
- `grep` の全機能を置き換えない。構造を持つファイルに限る
- YAML 1.2 の完全実装。**部分集合を明示して守る**
- TOML / JSON は v1 に含めない（問題は同じなので、YAML が動いてから同じ API で足す）

---

## D-2 実測

2026-09-02 / rustc 1.98.0 / x86_64-linux。同じ fixture（`scopegrep-core/testdata/workflow-with-comment.yml`）を
各 crate の最小プログラムで読み、`cancelled()` を含むスカラーを列挙した。
使い捨ての PoC で、コードはリポジトリに入れていない。**測っていない項目は空欄のまま。**

fixture の `grep -n 'cancelled()'` は **5 行**返す（4・29・30 行目がコメント、33・46 行目が値）。
行ベースの検索の偽陽性は 3 件である。

| | `saphyr-parser` 0.0.12 | `marked-yaml` 0.8.0 | `yaml-rust2` 0.12.0 | `serde_yaml` 0.9.34 | `tree-sitter-yaml` 0.7.2 | `noyalib` 0.0.29 |
| --- | --- | --- | --- | --- | --- | --- |
| 値の 2 件だけを返す | ✅ | ✅ | ✅ | ✅（**行番号なし**） | ✅ ＋ コメント 3 件を別枠 | ✅ ＋ コメント 3 件を別枠 |
| スカラーの位置 | あり（`Span` start/end） | あり（start のみ） | あり（start のみ） | **なし**（エラー時のみ） | あり | あり |
| コメントの露出 | なし | なし | なし | なし | **あり** | **あり**（`comments_at(path)`） |
| 推移的依存（自身除く） | 8 | 9 | 7 | 9 | 9 ＋ `cc` | 11 |
| `build.rs` / C コンパイラ | 3 件 / 不要 | 1 件 / 不要 | **0 件** / 不要 | 2 件 / 不要 | **2 件 / 必須**（`cc`） | 1 件 / 不要 |
| `no_std`（`thumbv7m-none-eabi` で実ビルド） | **通る** | 落ちる（`arraydeque`） | 落ちる（`arraydeque`） | 落ちる（`serde_core`） | 未検証（クロス C コンパイラ無し） | 未検証 |
| クリーンビルド（release） | 4.0 s | 4.0 s | 4.1 s | 4.1 s | 9.2 s | 7.3 s |
| バイナリ（strip なし） | 606 KB | 719 KB | 627 KB | 743 KB | 858 KB | 922 KB |
| 反復順 | 文書順 | 挿入順 | 挿入順 | 挿入順 | 文書順 | 挿入順 |

**実測で分かった癖（想定には無かったもの）:**

- `saphyr-parser` と `yaml-rust2` の `Marker::col()` は **doc コメントが「1-indexed」と言いながら 0 始まりを返す**。
  位置を扱う道具がこれを信じると全ヒットの列が 1 ずれる
- `tree-sitter-yaml` の構文木では、**ステップの直前に書いたコメントが前のステップの子になる**
  （29〜30 行目のコメントは `steps[3]` の説明だが、木では `steps[2]` に付く）。
  「このコメントは誰の説明か」を出すには後処理が要る。`noyalib` はそれを `comments_at` で解決している
- `serde_yml` の最新版 0.0.13 は crates.io の説明文どおり **`noyalib` へ転送するだけの shim**。選ぶ意味が無い
- `marked-yaml` はトップレベルが mapping か sequence でなければならず、alias / anchor を受け付けないと明記している

**測っていないこと:** alias / anchor を含む YAML・複数ドキュメント・パース速度・メモリ・strip 後のサイズ。
