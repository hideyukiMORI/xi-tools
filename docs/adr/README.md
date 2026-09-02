# Architecture Decision Records

長期に効く判断を1件1ファイルで残す。`0000-template.md` を複製して書く。

**ADR が要る場面**（`docs/coding-rules.md` QLT-005 が指定する）:

- ゲートを弱めるとき（閾値の緩和・`forbid` から `deny` への降格・lint の削除・免除の追加・規則の降格）
- 外部依存を足すとき（ARC-004）
- ツール間でコードを共有するとき（ARC-001）
- 複雑度の上限を超える必要があるとき（RS-011）
- 新しいツールを足すとき

🔴 **ADR は「決めたこと」だけでなく「却下したこと」を残す文書である。**
却下の理由が無いと、同じ提案が周期的に戻ってくる。

## 索引

| ADR | 題 | Status |
| --- | --- | --- |
| [0001](0001-strictness-is-mechanically-enforced.md) | 厳格性は文章ではなく機械で強制する | accepted (2026-09-02) |
| [0002](0002-regex-is-an-opt-in-feature.md) | 正規表現は opt-in の feature で足し、既定の依存は 0 のまま保つ | accepted (2026-09-02) |
| [0003](0003-fleet-top-fetches-github-via-chunked-graphql.md) | `fleet-top` を足す。GitHub は分割した GraphQL を並列に叩き、依存は 0 のまま保つ | accepted (2026-09-02) |
