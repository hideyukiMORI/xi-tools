//! GitHub に問い合わせるかどうか。

/// GitHub に問い合わせるかどうか（`--no-github`）。
///
/// 🔑 `bool` にしない（RS-002）。`true` が「聞く」なのか「聞かない」なのかは
/// 呼び出し側の名前でしか決まらず、旗の綴りが否定形（`--no-github`）である以上、
/// **どちらの向きにも読める**。閉じた選択肢は enum で持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GithubAccess {
    /// `gh api graphql` を叩く（既定）。
    Query,
    /// 叩かない。GitHub の 3 列は `n/a` になる。
    Skip,
}

#[cfg(test)]
mod tests {
    use super::GithubAccess;

    #[test]
    fn the_two_answers_are_distinct() {
        assert_ne!(GithubAccess::Query, GithubAccess::Skip);
    }
}
