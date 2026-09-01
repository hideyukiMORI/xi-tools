//! scopegrep — ヒットした行が、構造のどこに属するかを返す。
//!
//! `grep` は「その行がある」ことしか返さない。YAML の入れ子はテキストの行番号に
//! 現れないので、`.github/workflows/*.yml` を検索しても
//! **その条件がどのステップに付いているか** は分からない。
//!
//! 実装は未着手。設計は `docs/design/scopegrep.md`。

fn main() {
    eprintln!("scopegrep: not implemented yet — see docs/design/scopegrep.md");
    std::process::exit(2);
}

#[cfg(test)]
mod tests {
    /// 足場が CI で実際に走ることを確かめるためだけのテスト。
    /// 実装が入ったら消す（残すと「テストが在る」という嘘の緑になる）。
    #[test]
    fn scaffold_builds() {
        assert_eq!(2 + 2, 4);
    }
}
