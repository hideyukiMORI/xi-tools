//! 壊れた入力の種別。

use core::fmt;

/// 部分集合の内側でも読めない形。**構文としておかしいもの**を表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalformedInput {
    /// タブによるインデント。YAML はインデントにタブを許さない。
    TabIndentation,
    /// インデントの矛盾（親より浅い位置に子が来る等）。
    InconsistentIndentation,
    /// クォートやフロー記法の後ろに、コメントでない文字が続く。
    TrailingContent,
    /// ブロックスカラーの指示子（`|` / `>` の後ろ）が読めない。
    BlockScalarHeader,
}

impl fmt::Display for MalformedInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match *self {
            Self::TabIndentation => "the indentation contains a tab",
            Self::InconsistentIndentation => "the indentation does not line up",
            Self::TrailingContent => "trailing characters after the value",
            Self::BlockScalarHeader => "cannot read the block scalar indicator",
        };
        f.write_str(text)
    }
}
