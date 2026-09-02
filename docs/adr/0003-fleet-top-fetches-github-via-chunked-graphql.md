# ADR 0003: `fleet-top` を足す。GitHub は分割した GraphQL を並列に叩き、依存は 0 のまま保つ

## Status

accepted (2026-09-02・施主指示「開発開始」)

## Context

数十のリポジトリの状態（枝・未コミット・ahead/behind・open PR・CI・古い枝）を 1 画面で見たい。
手で回すと `gh api` が直列で 1 本 0.74 秒、42 リポ × 3 本で 93 秒（外挿）。**打たれない長さ**である
（設計メモ [`docs/design/fleet-top.md`](../design/fleet-top.md) の「解く問題」）。

`docs/adr/README.md` は「新しいツールを足すとき」を ADR の対象に含める。本 ADR がそれである。
候補一覧（[`docs/design/candidates.md`](../design/candidates.md)）の採用条件 3 つ——実際に踏んだ穴／手元の実データで実測できる／1 本で閉じる——を満たす。

作る前に 1 時間の試作で並列化の効果を測った（数字は設計メモの「実測」節）。要点:

| 形 | 60 リポ |
| --- | --- |
| REST 64 並列 | 2.38 s（42 リポ・126 本）・rate limit 126 点 |
| GraphQL 1 本 | 8.87 s（42 リポ）／ 502（60 リポ） |
| **GraphQL 3 リポ × 20 本を並列** | **1.35〜1.49 s**・20 点 |

## Decision

1. **`fleet-top` を workspace に足す。** `fleet-top-core`（`#![no_std]`・依存 0）と `fleet-top`（bin・依存 0）の 2 クレート。
   `scopegrep` と同じ形で、`cargo publish -p` で単独 publish できる形を保つ
2. **GitHub は `gh api graphql` をサブプロセスで叩く。** 3 リポを 1 リクエストにまとめ、全リクエストを並列に投げる。
   認証は `gh` から借り、token を扱わない
3. **並行実行は `std::thread::scope`。** 依存を足さない。ワーカー上限 32
4. **JSON の読み取りは `fleet-top-core` に手書きする**（RFC 8259）。`serde_json` を入れない
5. **失敗は `?` として表に出し、終了コード 1 で伝える。** 行を消さない。`gh api graphql` が終了コード 1 でも stdout の `data` を読み、
   `errors[].path` が指すリポだけを失敗にする
6. **`scopegrep-core` とコードを共有しない**（ARC-001）。JSON パーサが `scopegrep` で要るようになったら、そのとき ADR を立てる

### 却下した選択肢

| 案 | 却下の理由 |
| --- | --- |
| REST を高並列で叩く | 64 並列でも 2.38 s。rate limit の消費が GraphQL 分割の 6 倍 |
| GraphQL 1 本に全リポ | まとめるほど遅い（42 リポ 8.87 s、60 リポで 502）。サーバ側で直列に解決している |
| `octocrab` / `reqwest` で直接 API | token の置き場・TLS・`tokio` を抱える。`gh` が既に認証を持っている |
| `curl` 直叩き | `gh api` より 1 本 0.25 s 速いが、token を自分で扱うことになる。並列にすれば差は消える |
| `tokio` | サブプロセスの待ち合わせに非同期ランタイムは要らない。試作は `std` だけで 1.4 s |
| `serde_json` | 5 crate 入り、core が `alloc` だけで閉じなくなる。JSON は仕様が小さく、fixture で全部試験できる |
| `ratatui` の TUI | 1.5 秒で返る道具を常駐させる理由が無い。`watch fleet-top` で足りる |
| `gh api --jq` で平たくしてから読む | 構造の解釈が jq 文字列に隠れ、タイトルの改行・タブで壊れる |
| `scopegrep-core` に JSON を置いて共有 | ARC-001。共有は結合であり、`scopegrep` に要らないコードが入る |

## Consequences

- `deny.toml` の `allow` は変えない（依存 0 なので新しいライセンスは入らない）
- ARC-003 の適用範囲が広がる。「I/O が本体の道具でも、I/O の**結果**を文字列で受ける中核は `no_std` にできる」が前例になる
- README の例は `tests/readme.rs` で照合できない（出力が時刻とネットワークに依存する）。表の整形は core の fixture テストで完全一致を見る。
  **この例外を `scopegrep` に逆流させない**（`scopegrep` の例は今までどおり実行して照合する）
- QLT-009（性能の主張は実測を伴う）が初めて効く。README に書く数字は設計メモの実測表からだけ取る
- `docs/coding-rules.md` の ARC-001 は「現在ツールは `scopegrep` 1 つ」と書いている。2 つになるので、その記述を更新する
