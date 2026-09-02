//! 表の整形（列幅・並び順）。

use alloc::string::String;
use alloc::vec::Vec;

use crate::freshness::Freshness;
use crate::row::Row;

/// 列の数。
pub(crate) const COLUMN_COUNT: usize = 7;

/// 見出し。列の並びはこの順で固定である。
const HEADINGS: [&str; COLUMN_COUNT] = [
    "REPO",
    "BRANCH",
    "DIRTY",
    "AHEAD/BEHIND",
    "PR",
    "CI",
    "STALE",
];

/// 列と列の間。
const SEPARATOR: &str = "  ";

/// 表を組む。行は名前の**バイト順**に並べ替える。
///
/// 🔴 **入力の順に依存しない。** bin は並列に集めた結果を任意の順で渡してくるので、
/// ここで並べ替えないと実行ごとに表が変わり、差分が取れなくなる（RS-016）。
/// 同じ名前の行は入力順のまま残る（安定な並べ替え）。
///
/// 列幅はその列の最大**文字数**（見出しを含む）で、最終列は詰めない。
/// 各行は `\n` で終わり、**行末に空白は出さない**。行が 0 でも見出しは出る。
#[must_use]
pub fn render(rows: &[Row], freshness: &Freshness) -> String {
    let lines = cell_lines(rows, freshness);
    let widths = column_widths(&lines);
    let mut table = String::new();
    for cells in &lines {
        push_line(&mut table, cells, &widths);
    }
    table
}

/// 見出しと、名前順に並べた各行のセル。
fn cell_lines(rows: &[Row], freshness: &Freshness) -> Vec<[String; COLUMN_COUNT]> {
    let mut ordered: Vec<&Row> = rows.iter().collect();
    // 🔑 `sort_by` は安定なので、同じ名前の行は入力順のまま残る。
    ordered.sort_by(|left, right| left.name().as_bytes().cmp(right.name().as_bytes()));

    let mut lines = Vec::with_capacity(ordered.len().saturating_add(1_usize));
    lines.push(HEADINGS.map(String::from));
    lines.extend(ordered.into_iter().map(|row| row.cells(freshness)));
    lines
}

/// 各列の幅（その列の最大文字数）。
///
/// 🔴 **バイト数ではなく文字数で数える。** 枝名に日本語が入っても列が崩れない
/// （端末上の見た目の幅は v1 では扱わない。全角も 1 文字と数える）。
fn column_widths(lines: &[[String; COLUMN_COUNT]]) -> [usize; COLUMN_COUNT] {
    let mut widths = [0_usize; COLUMN_COUNT];
    for cells in lines {
        for (width, cell) in widths.iter_mut().zip(cells.iter()) {
            *width = (*width).max(cell.chars().count());
        }
    }
    widths
}

/// 1 行ぶんを書き出す。最終列は詰めないので、行末に空白が出ない。
fn push_line(table: &mut String, cells: &[String; COLUMN_COUNT], widths: &[usize; COLUMN_COUNT]) {
    let last = COLUMN_COUNT.saturating_sub(1_usize);
    for (index, (cell, width)) in cells.iter().zip(widths.iter()).enumerate() {
        if index != 0_usize {
            table.push_str(SEPARATOR);
        }
        table.push_str(cell);
        if index != last {
            for _ in 0_usize..width.saturating_sub(cell.chars().count()) {
                table.push(' ');
            }
        }
    }
    table.push('\n');
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::branch_list::BranchList;
    use crate::ci_state::CiState;
    use crate::day::Day;
    use crate::freshness::Freshness;
    use crate::local_report::LocalReport;
    use crate::local_state::parse_porcelain;
    use crate::remote_branch::RemoteBranch;
    use crate::remote_report::RemoteReport;
    use crate::remote_state::RemoteState;
    use crate::row::Row;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    fn freshness() -> Freshness {
        Freshness::new(Day::parse_iso8601("2026-09-02").expect("読める"), 30_u32)
    }

    fn day(text: &str) -> Day {
        Day::parse_iso8601(text).expect("読めるはずである")
    }

    fn local(source: &str) -> LocalReport {
        LocalReport::State(parse_porcelain(source).expect("読めるはずである"))
    }

    /// 古い枝を `stale` 本持つ GitHub 側の状態。
    fn remote(pull_requests: u32, ci: CiState, stale: usize) -> RemoteReport {
        let mut branches = vec![RemoteBranch::new(String::from("main"), day("2026-09-01"))];
        for index in 0_usize..stale {
            let name = alloc::format!("old-{index}");
            branches.push(RemoteBranch::new(name, day("2026-01-01")));
        }
        RemoteReport::State(RemoteState::new(
            Some(String::from("main")),
            ci,
            pull_requests,
            BranchList::new(branches, false),
        ))
    }

    /// 設計メモの例の 4 行。
    ///
    /// ⚠️ 設計メモの表は `delta` の `BRANCH` を `main`、`DIRTY` を `?` と書いているが、
    /// [`LocalReport`] は「読めた（枝も dirty も在る）」か「取れなかった（両方 `?`）」の
    /// どちらかしか表せないので、**両立しない**。作業指示の「`delta` は local も remote も
    /// `Unavailable`」に従い、`delta` の行は全て `?` にした。
    /// また並び順は**バイト順**なので `delta` が `gamma` より前に来る（設計メモの例は
    /// ギリシャ文字の順で並んでいる）。どちらも列幅には影響しない。
    fn example() -> Vec<Row> {
        vec![
            Row::new(
                String::from("alpha"),
                local("# branch.head main\n# branch.upstream origin/main\n# branch.ab +0 -0\n"),
                remote(0_u32, CiState::Success, 0_usize),
            ),
            Row::new(
                String::from("beta"),
                local(
                    "# branch.head feat/login\n# branch.upstream origin/feat/login\n# branch.ab +2 -1\n? a\n? b\n? c\n",
                ),
                remote(1_u32, CiState::Failure, 2_usize),
            ),
            Row::new(
                String::from("gamma"),
                local("# branch.head (detached)\n"),
                RemoteReport::NotOnGithub,
            ),
            Row::new(
                String::from("delta"),
                LocalReport::Unavailable,
                RemoteReport::Unavailable,
            ),
        ]
    }

    /// 🔴 設計メモ「出力の形」と完全一致する（上の注記の 2 点を除く）。
    #[test]
    fn renders_the_example_from_the_design_note() {
        let expected = "\
REPO   BRANCH      DIRTY  AHEAD/BEHIND  PR   CI    STALE
alpha  main        -      -             -    ok    -
beta   feat/login  3      +2/-1         1    FAIL  2
delta  ?           ?      ?             ?    ?     ?
gamma  (detached)  -      (none)        n/a  n/a   n/a
";
        assert_eq!(render(&example(), &freshness()), expected);
    }

    /// 入力の順に依存しない。逆順で渡しても同じ表が出る。
    #[test]
    fn the_order_of_the_input_does_not_matter() {
        let mut reversed = example();
        reversed.reverse();
        assert_eq!(
            render(&reversed, &freshness()),
            render(&example(), &freshness())
        );
    }

    /// 行が 0 でも見出しは出る（**黙って空にしない**）。
    #[test]
    fn an_empty_table_still_has_its_headings() {
        assert_eq!(
            render(&[], &freshness()),
            "REPO  BRANCH  DIRTY  AHEAD/BEHIND  PR  CI  STALE\n"
        );
    }

    /// 行末に空白を出さない（最終列を詰めない）。
    #[test]
    fn no_line_ends_with_a_space() {
        let table = render(&example(), &freshness());
        for line in table.lines() {
            assert!(!line.ends_with(' '), "{line} の行末に空白がある");
        }
    }

    /// 🔴 列幅は**文字数**で揃える。全角の枝名でも列が崩れない
    /// （端末上の見た目の幅は v1 では扱わない）。
    #[test]
    fn the_width_counts_characters_not_bytes() {
        let rows = vec![
            Row::new(
                String::from("alpha"),
                local("# branch.head 機能/ログイン\n"),
                RemoteReport::NotOnGithub,
            ),
            Row::new(
                String::from("beta"),
                local("# branch.head main\n"),
                RemoteReport::NotOnGithub,
            ),
        ];
        let table = render(&rows, &freshness());
        // 🔑 `機能/ログイン` は 7 文字（バイト数は 19）。`BRANCH` 列の幅は 7 になり、
        //    `main` は 3 つの空白で詰められる。端末上の見た目は全角ぶん広がるが、
        //    それは v1 では扱わない（設計メモ「出力の形」）。
        let expected = "\
REPO   BRANCH   DIRTY  AHEAD/BEHIND  PR   CI   STALE
alpha  機能/ログイン  -      (none)        n/a  n/a  n/a
beta   main     -      (none)        n/a  n/a  n/a
";
        assert_eq!(table, expected);
    }

    /// 同じ名前の行は入力順のまま残る（安定な並べ替え）。
    #[test]
    fn rows_with_the_same_name_keep_their_order() {
        let rows = vec![
            Row::new(
                String::from("alpha"),
                local("# branch.head first\n"),
                RemoteReport::NotOnGithub,
            ),
            Row::new(
                String::from("alpha"),
                local("# branch.head second\n"),
                RemoteReport::NotOnGithub,
            ),
        ];
        let table = render(&rows, &freshness());
        let first = table.lines().nth(1_usize).expect("2 行目が在る");
        assert!(first.contains("first"), "{first} が先に来ていない");
    }

    /// 名前はバイト順に並ぶ（大文字は小文字より前）。
    #[test]
    fn names_are_ordered_by_bytes() {
        let names = ["b", "A", "a", "B"];
        let rows: Vec<Row> = names
            .iter()
            .map(|name| {
                Row::new(
                    String::from(*name),
                    LocalReport::Unavailable,
                    RemoteReport::Unavailable,
                )
            })
            .collect();
        let table = render(&rows, &freshness());
        let ordered: Vec<&str> = table
            .lines()
            .skip(1_usize)
            .filter_map(|line| line.split(' ').next())
            .collect();
        assert_eq!(ordered, vec!["A", "B", "a", "b"]);
    }

    /// pending の CI は `...` で出る。
    #[test]
    fn a_pending_ci_is_marked_with_dots() {
        let rows = vec![Row::new(
            String::from("alpha"),
            local("# branch.head main\n"),
            remote(0_u32, CiState::Pending, 0_usize),
        )];
        assert!(render(&rows, &freshness()).contains("..."));
    }
}
