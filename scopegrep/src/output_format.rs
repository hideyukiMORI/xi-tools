//! 出力の形。

/// 出力の形。既定は人向けの1行で、`--json` で機械向けになる（設計メモ D-4）。
///
/// 🔑 `--show-scope` のような「所属を出さないモード」は**設けない**。
/// 所属を出すことがこの道具の存在理由であり、出さない形は要らない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    /// `<file>:<line>: <path> = <value>`。`grep -n` と同じ頭を持つ。
    Human,
    /// 1ヒット1行の JSON。途中で切れても壊れない。
    Json,
}
