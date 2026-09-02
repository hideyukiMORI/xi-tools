//! 対応していない YAML 構文の種別。

use core::fmt;

/// 読める部分集合の外にある構文。
///
/// 🔴 **黙って誤読しない**ための型である（設計メモ D-2）。
/// 「読めなかった」と言うほうが、間違った所属を返すより道具として誠実である。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedSyntax {
    /// アンカー `&name`。
    Anchor,
    /// エイリアス `*name`。
    Alias,
    /// マージキー `<<:`。
    MergeKey,
    /// 複合キー `? `。
    ComplexKey,
    /// 複数行にまたがるスカラー（プレーンの継続行・閉じないクォート）。
    MultiLineScalar,
    /// 閉じていないフロー記法（`[` や `{` が最後まで閉じない）。
    ///
    /// 🔑 **複数行にまたがること自体は読める**（v1.1）。読めないのは、
    /// 桁が親まで戻っても・ファイルが終わっても閉じ括弧が来ない場合である。
    UnclosedFlow,
    /// 2つ目の `---` / `...`（複数ドキュメント）。
    MultipleDocuments,
    /// `%YAML` などのディレクティブ。
    Directive,
    /// 1行に入れ子で書かれたシーケンス（`- - a`）。
    NestedInlineSequence,
}

impl fmt::Display for UnsupportedSyntax {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match *self {
            Self::Anchor => "an anchor (&name)",
            Self::Alias => "an alias (*name)",
            Self::MergeKey => "a merge key (<<:)",
            Self::ComplexKey => "a complex key (? )",
            Self::MultiLineScalar => "a multi-line scalar",
            Self::UnclosedFlow => "an unclosed flow style ([ or { never closes)",
            Self::MultipleDocuments => "a second document (--- / ...)",
            Self::Directive => "a directive (%YAML and the like)",
            Self::NestedInlineSequence => "a sequence nested on one line (- - a)",
        };
        f.write_str(text)
    }
}
