//! 手元のリポジトリの状態（`git status --porcelain=v2 --branch` の読み取り）。

use alloc::string::String;

use crate::divergence::Divergence;
use crate::head::Head;
use crate::porcelain_error::PorcelainError;
use crate::porcelain_error_kind::PorcelainErrorKind;
use crate::porcelain_line::PorcelainLine;

/// 手元のリポジトリの状態。
///
/// フィールドは非公開で、生成経路は [`parse_porcelain`] だけである（RS-001 / RS-003）。
/// **`git` の出力を保持しない**——読んだ結果だけを持つので、この型を受け取った側は
/// もう一度 porcelain を解釈しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalState {
    head: Head,
    upstream: Option<String>,
    divergence: Divergence,
    dirty: u32,
}

impl LocalState {
    /// 読み取った各値から作る。
    pub(crate) fn new(
        head: Head,
        upstream: Option<String>,
        divergence: Divergence,
        dirty: u32,
    ) -> Self {
        Self {
            head,
            upstream,
            divergence,
            dirty,
        }
    }

    /// いま居る場所（枝か detached か）。
    #[must_use]
    pub fn head(&self) -> &Head {
        &self.head
    }

    /// 上流の追跡枝（`# branch.upstream` の値）。設定されていなければ `None`。
    #[must_use]
    pub fn upstream(&self) -> Option<&str> {
        self.upstream.as_deref()
    }

    /// 上流より進んでいるコミット数。上流が無ければ 0。
    #[must_use]
    pub fn ahead(&self) -> u32 {
        self.divergence.ahead()
    }

    /// 上流より遅れているコミット数。上流が無ければ 0。
    #[must_use]
    pub fn behind(&self) -> u32 {
        self.divergence.behind()
    }

    /// 変更・改名・衝突・未追跡の**エントリ数**。無視されたファイルは数えない。
    #[must_use]
    pub fn dirty(&self) -> u32 {
        self.dirty
    }
}

/// `git status --porcelain=v2 --branch` の出力を読む。
///
/// 末尾の改行の有無は問わない。`\r\n` も受ける（`str::lines` が `\r` を落とす）。
///
/// # Errors
///
/// `# branch.head` が無い、`#` 見出しの値が読めない、porcelain v2 の行として
/// 読めない行がある、のいずれかで [`PorcelainError`] を返す。エラーは**必ず行番号を持つ**。
pub fn parse_porcelain(source: &str) -> Result<LocalState, PorcelainError> {
    let mut head: Option<Head> = None;
    let mut upstream: Option<String> = None;
    // `# branch.ab` の行が無いとき（上流が無いとき）は 0 と 0 である。
    let mut divergence = Divergence::new(0_u32, 0_u32);
    let mut dirty = 0_u32;

    for (index, line) in source.lines().enumerate() {
        let number = u32::try_from(index.saturating_add(1_usize)).unwrap_or(u32::MAX);
        let read = PorcelainLine::read(line).map_err(|kind| PorcelainError::new(kind, number))?;
        match read {
            PorcelainLine::Head(found) => head = Some(found),
            PorcelainLine::Upstream(name) => upstream = Some(name),
            PorcelainLine::Divergence(found) => divergence = found,
            PorcelainLine::Dirty => dirty = dirty.saturating_add(1_u32),
            PorcelainLine::Ignored => {}
        }
    }

    let head = head.ok_or_else(|| PorcelainError::new(PorcelainErrorKind::MissingHead, 0_u32))?;
    Ok(LocalState::new(head, upstream, divergence, dirty))
}

#[cfg(test)]
mod tests {
    use super::{LocalState, parse_porcelain};
    use crate::head::Head;
    use crate::porcelain_error_kind::PorcelainErrorKind;
    use alloc::string::String;

    /// 実測した形 1: コミットがまだ無いリポジトリ。
    const INITIAL: &str = "\
# branch.oid (initial)
# branch.head master
";

    /// 実測した形 2: detached HEAD ＋ 変更あり。
    const DETACHED: &str = "\
# branch.oid 5c2528bb47268df1e88c70244a03e2ba0af243cc
# branch.head (detached)
1 A. N... 000000 100644 100644 0000000000000000000000000000000000000000 78981922613b2afb6025042ff6bd878ac1994e85 a
? b
";

    /// 実測した形 3: 上流あり ＋ 行の種類が全部出ている。
    const TRACKED: &str = "\
# branch.oid be1ac856ed7b0fda91270b20c022e7bda6bf8206
# branch.head main
# branch.upstream origin/main
# branch.ab +0 -0
1 .M N... 100644 100644 100644 a7c9904d179471e47f7ef58ee8afbbcd0f3eac72 a7c9904d179471e47f7ef58ee8afbbcd0f3eac72 notes.md
2 R. N... 100644 100644 100644 0cbf1228461a5f32eaaeaae6663ba5a9147d6598 0cbf1228461a5f32eaaeaae6663ba5a9147d6598 R100 new.md\told.md
u UU N... 100644 100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 3333333333333333333333333333333333333333 conflict.txt
? scratch/
! ignored.log
";

    fn read(source: &str) -> LocalState {
        parse_porcelain(source).expect("読めるはずである")
    }

    #[test]
    fn reads_a_repository_without_commits() {
        let state = read(INITIAL);
        assert_eq!(*state.head(), Head::Branch(String::from("master")));
        assert_eq!(state.upstream(), None);
        assert_eq!(state.ahead(), 0_u32);
        assert_eq!(state.behind(), 0_u32);
        assert_eq!(state.dirty(), 0_u32);
    }

    #[test]
    fn reads_a_detached_head_with_changes() {
        let state = read(DETACHED);
        assert_eq!(*state.head(), Head::Detached);
        assert_eq!(state.upstream(), None);
        assert_eq!(state.dirty(), 2_u32);
    }

    /// 🔴 `!`（無視されたファイル）は dirty に数えない。数えると、
    /// `.gitignore` を書いただけのリポジトリが全部「汚れている」ことになる。
    #[test]
    fn reads_a_tracked_branch_and_skips_ignored_files() {
        let state = read(TRACKED);
        assert_eq!(*state.head(), Head::Branch(String::from("main")));
        assert_eq!(state.upstream(), Some("origin/main"));
        assert_eq!(state.ahead(), 0_u32);
        assert_eq!(state.behind(), 0_u32);
        assert_eq!(state.dirty(), 4_u32);
    }

    #[test]
    fn reads_the_ahead_behind_counts() {
        let source = "# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n";
        let state = read(source);
        assert_eq!(state.ahead(), 2_u32);
        assert_eq!(state.behind(), 1_u32);
    }

    /// `\r\n` の入力と、末尾に改行が無い入力を同じに読む。
    #[test]
    fn reads_crlf_and_a_missing_final_newline() {
        let expected = read(INITIAL);
        assert_eq!(
            read("# branch.oid (initial)\r\n# branch.head master\r\n"),
            expected
        );
        assert_eq!(
            read("# branch.oid (initial)\n# branch.head master"),
            expected
        );
    }

    #[test]
    fn empty_input_has_no_head() {
        let error = parse_porcelain("").expect_err("読めないはずである");
        assert_eq!(error.kind(), &PorcelainErrorKind::MissingHead);
        assert_eq!(error.line(), 0_u32);
    }

    /// 見出しはあるが `# branch.head` が無い（`--branch` を忘れた出力）。
    #[test]
    fn a_status_without_the_branch_option_has_no_head() {
        let error = parse_porcelain("? scratch/\n").expect_err("読めないはずである");
        assert_eq!(error.kind(), &PorcelainErrorKind::MissingHead);
    }

    #[test]
    fn reports_the_line_of_a_malformed_header() {
        let source = "# branch.head main\n# branch.upstream origin/main\n# branch.ab +x -1\n";
        let error = parse_porcelain(source).expect_err("読めないはずである");
        assert_eq!(error.kind(), &PorcelainErrorKind::MalformedHeader);
        assert_eq!(error.line(), 3_u32);
    }

    #[test]
    fn reports_the_line_of_an_unexpected_line() {
        let source = "# branch.head main\nz something\n";
        let error = parse_porcelain(source).expect_err("読めないはずである");
        assert_eq!(error.kind(), &PorcelainErrorKind::UnexpectedLine);
        assert_eq!(error.line(), 2_u32);
    }

    /// 知らない見出しは無視する。`# stash 2` も同じ。
    #[test]
    fn unknown_headers_do_not_break_the_read() {
        let source = "# branch.head main\n# branch.future something\n# stash 2\n";
        let state = read(source);
        assert_eq!(*state.head(), Head::Branch(String::from("main")));
        assert_eq!(state.dirty(), 0_u32);
    }

    /// 枝名は原文のまま。`/` を含む名前を割らない。
    #[test]
    fn keeps_the_branch_name_verbatim() {
        let state = read("# branch.head feat/login\n");
        assert_eq!(*state.head(), Head::Branch(String::from("feat/login")));
    }

    /// 後から来た見出しが勝つ（同じ見出しが 2 度出ることは無いが、黙って壊れない）。
    #[test]
    fn a_later_header_wins() {
        let state = read("# branch.head main\n# branch.head other\n");
        assert_eq!(*state.head(), Head::Branch(String::from("other")));
    }
}
