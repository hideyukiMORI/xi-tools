//! GitHub に聞くリポジトリ1つ（表の何行目か、と owner/name）。

use fleet_top_core::github_slug::GithubSlug;

/// GitHub に聞く対象1つ。
///
/// 🔑 `(usize, GithubSlug)` のタプルで持ち回らない（RS-006）。この `index` は
/// **表の何行目に結果を戻すか**であって、塊の中の位置でも問い合わせ順でもない。
/// 意味のある値に名前を付けておかないと、`0` を渡す場所を1つ間違えても型が黙る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    index: usize,
    slug: GithubSlug,
}

impl Target {
    /// 表の行番号（0 起点）と owner/name から作る。
    pub(crate) fn new(index: usize, slug: GithubSlug) -> Self {
        Self { index, slug }
    }

    /// 結果を戻す行（0 起点）。
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    /// GitHub の owner/name。
    pub(crate) fn slug(&self) -> &GithubSlug {
        &self.slug
    }
}
