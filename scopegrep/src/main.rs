//! scopegrep — ヒットした行が、構造のどこに属するかを返す。
//!
//! `grep` は「その行がある」ことしか返さない。YAML の入れ子はテキストの行番号に
//! 現れないので、`.github/workflows/*.yml` を検索しても
//! **その条件がどのステップに付いているか** は分からない。
//!
//! 実装は未着手。設計は `docs/design/scopegrep.md`。

/// 未実装であることを示す終了コード。`grep` の「一致なし」(1) と区別する。
const EXIT_NOT_IMPLEMENTED: i32 = 2;

// 🔴 出力を行ってよい唯一の場所（RS-014）。この #[expect] が「ここだけが標準
//    エラーに書く」という宣言そのものである。実装が入って出力層を切り出したら、
//    この抑制はそちらへ移る。ここに残したまま出力を増やさないこと。
#[expect(
    clippy::print_stderr,
    reason = "RS-014: 出力は1箇所に集約する。足場の段階では main が唯一の出力点である"
)]
fn main() {
    eprintln!("scopegrep: not implemented yet — see docs/design/scopegrep.md");
    std::process::exit(EXIT_NOT_IMPLEMENTED);
}

#[cfg(test)]
mod tests {
    use super::EXIT_NOT_IMPLEMENTED;

    /// 足場が CI で実際に走ることを確かめるためだけのテスト。
    /// 実装が入ったら消す（残すと「テストが在る」という嘘の緑になる）。
    #[test]
    fn scaffold_builds() {
        assert_eq!(EXIT_NOT_IMPLEMENTED, 2_i32);
    }
}
