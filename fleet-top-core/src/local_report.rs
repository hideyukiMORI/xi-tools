//! 手元の状態について言えること。

use crate::local_state::LocalState;

/// 1 リポジトリの手元の状態について、表に出せること。
///
/// 🔴 **取れなかったことを「変更なし」で表さない。** `git` が失敗したリポジトリを
/// [`LocalState`] のゼロ値で埋めると、表では「きれいな main」に見える。
/// この道具が生まれた事故（片方だけ見て判断した）と同じ形なので、
/// **取れなかったことは型に出す**（設計メモ F-5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalReport {
    /// `git` の出力を読めた。
    State(LocalState),
    /// 取れなかった（`git` が無い・失敗した・出力を読めなかった）。表では `?`。
    Unavailable,
}

#[cfg(test)]
mod tests {
    use super::LocalReport;
    use crate::local_state::parse_porcelain;

    #[test]
    fn a_state_is_not_the_absence_of_one() {
        let state = parse_porcelain("# branch.head main\n").expect("読めるはずである");
        assert_ne!(LocalReport::State(state), LocalReport::Unavailable);
    }
}
