//! いま居る場所（枝か、detached か）。

use alloc::string::String;

/// `git status --porcelain=v2 --branch` の `# branch.head` が言っていること。
///
/// 🔑 **detached を「枝名が空の枝」で表さない。** 空文字列にすると、表の
/// `BRANCH` 列で「取れなかった（`?`）」「枝が無い」「名前が空」の 3 つが
/// 同じ見た目になる。区別が要るものは enum で持つ（RS-002 / RS-004）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    /// 枝の上に居る。名前は **git が書いたまま**で、`feat/login` のように `/` を含む。
    Branch(String),
    /// どの枝の上にも居ない（`# branch.head (detached)`）。
    Detached,
}

#[cfg(test)]
mod tests {
    use super::Head;
    use alloc::string::String;

    #[test]
    fn a_branch_keeps_its_name_verbatim() {
        let head = Head::Branch(String::from("feat/login"));
        assert_eq!(head, Head::Branch(String::from("feat/login")));
        assert_ne!(head, Head::Detached);
    }
}
