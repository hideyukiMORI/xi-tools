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
    /// タグ `!!str` / `!x`。
    Tag,
    /// マージキー `<<:`。
    MergeKey,
    /// 複合キー `? `。
    ComplexKey,
    /// 複数行にまたがるスカラー（プレーンの継続行・閉じないクォート）。
    MultiLineScalar,
    /// 複数行にまたがるフロー記法（`[` や `{` が行内で閉じない）。
    MultiLineFlow,
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
            Self::Anchor => "アンカー（&name）",
            Self::Alias => "エイリアス（*name）",
            Self::Tag => "タグ（!name）",
            Self::MergeKey => "マージキー（<<:）",
            Self::ComplexKey => "複合キー（? ）",
            Self::MultiLineScalar => "複数行にまたがるスカラー",
            Self::MultiLineFlow => "行内で閉じないフロー記法",
            Self::MultipleDocuments => "複数ドキュメント（2つ目の --- / ...）",
            Self::Directive => "ディレクティブ（%YAML 等）",
            Self::NestedInlineSequence => "1行に入れ子で書かれたシーケンス（- - a）",
        };
        f.write_str(text)
    }
}
