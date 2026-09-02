//! 読み終えた文書。

use alloc::vec::Vec;

use crate::comment_line::CommentLine;
use crate::hit::Hit;
use crate::scalar_line::ScalarLine;
use crate::search_scope::SearchScope;

/// 読み終えた文書。**内部表現は公開しない**（設計メモ「公開 API」）。
///
/// 中身は「スカラー1行 → その所属」と「コメント1行 → その所属」の平坦な表である。
/// 木で持つより、出現順がそのまま出力順になるほうがこの道具に合う（RS-016）。
///
/// 🔑 **コメントを捨てずに持つ**のがこの実装の存在理由である（設計メモ D-2）。
/// 値だけを読むパーサを選んでいたら、この表は作れなかった。
#[derive(Debug, Clone)]
pub struct Document {
    scalars: Vec<ScalarLine>,
    comments: Vec<CommentLine>,
}

impl Document {
    /// スカラーとコメントの表から作る。
    pub(crate) fn new(scalars: Vec<ScalarLine>, comments: Vec<CommentLine>) -> Self {
        Self { scalars, comments }
    }

    /// 固定文字列 `needle` を含む行を、出現順（行 → 桁）で返す。
    ///
    /// - `scope` が [`SearchScope::Values`] なら探すのは**スカラー値だけ**。
    ///   [`SearchScope::ValuesAndComments`] ならコメントも返すが、
    ///   どちらだったかは [`Hit::kind`] で必ず区別できる。キーは常に探さない
    /// - 1行に何度現れてもヒットは1件（`grep` と同じ行単位）。桁は最初の出現位置
    /// - 大文字小文字は区別する。正規表現ではない
    #[must_use]
    pub fn search(&self, needle: &str, scope: SearchScope) -> Vec<Hit> {
        let mut found: Vec<Hit> = self
            .scalars
            .iter()
            .filter_map(|scalar| scalar.find(needle))
            .collect();
        match scope {
            SearchScope::Values => {}
            SearchScope::ValuesAndComments => {
                found.extend(self.comments.iter().filter_map(|line| line.find(needle)));
                // 値とコメントは別の表にあるので、混ぜたら並べ直す。
                // 行末コメントは値と同じ行に出るため、行だけでは順序が決まらない。
                found.sort_by_key(|hit| (hit.line(), hit.column()));
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use crate::hit::Hit;
    use crate::hit_kind::HitKind;
    use crate::parse;
    use crate::search_scope::SearchScope;
    use alloc::format;
    use alloc::vec::Vec;

    /// D-2 の合否を決める fixture。手で書いた架空の CI 設定である。
    const WORKFLOW: &str = include_str!("../testdata/workflow-with-comment.yml");

    fn hits(source: &str, needle: &str) -> Vec<Hit> {
        parse(source)
            .expect("fixture は読める")
            .search(needle, SearchScope::Values)
    }

    fn lines(found: &[Hit]) -> Vec<u32> {
        found.iter().map(|hit| hit.line().get()).collect()
    }

    /// 🔴 この道具の存在理由そのもの。3つの `cancelled()` のうち、
    /// **コメント内の2件（29・30 行目）とヘッダの1件（4 行目）が落ちる**こと。
    #[test]
    fn finds_only_the_two_configuration_values() {
        let found = hits(WORKFLOW, "cancelled()");
        assert_eq!(found.len(), 2_usize, "コメント内を拾っていない");
        assert_eq!(lines(&found), [33_u32, 46_u32]);
        for hit in &found {
            let line = hit.line().get();
            assert_ne!(line, 4_u32, "ヘッダコメントを拾っている");
            assert_ne!(line, 29_u32, "散文のコメントを拾っている");
            assert_ne!(line, 30_u32, "散文のコメントを拾っている");
        }
    }

    #[test]
    fn reports_the_pointer_of_each_hit() {
        let found = hits(WORKFLOW, "cancelled()");
        let pointers: Vec<_> = found.iter().map(|hit| hit.path().pointer()).collect();
        assert_eq!(
            pointers,
            ["/jobs/frontend-check/steps/3/if", "/jobs/e2e/steps/2/if"]
        );
    }

    #[test]
    fn reports_the_human_readable_path_of_each_hit() {
        let found = hits(WORKFLOW, "cancelled()");
        let rendered: Vec<_> = found.iter().map(|hit| format!("{}", hit.path())).collect();
        assert_eq!(
            rendered,
            [
                "jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if",
                "jobs.e2e.steps[2] \"Upload Playwright report\" .if"
            ]
        );
    }

    #[test]
    fn reports_the_label_of_each_hit() {
        let found = hits(WORKFLOW, "cancelled()");
        let labels: Vec<_> = found.iter().map(|hit| hit.path().label()).collect();
        assert_eq!(
            labels,
            [
                Some("Audit (fail on high/critical)"),
                Some("Upload Playwright report")
            ]
        );
    }

    /// 値は原文のまま。桁は**一致が始まる位置**（値の先頭は 13 桁目、`cancelled()` は 18 桁目）。
    #[test]
    fn reports_the_value_and_the_column_of_the_match() {
        let found = hits(WORKFLOW, "cancelled()");
        let first = found.first().expect("1件目がある");
        assert_eq!(first.value(), "${{ !cancelled() }}");
        assert_eq!(first.column().get(), 18_u32);
    }

    /// キーは探さない。コメントも既定では探さない。
    #[test]
    fn searches_neither_keys_nor_comments() {
        assert!(hits(WORKFLOW, "frontend-check").is_empty());
        assert!(hits(WORKFLOW, "散文").is_empty());
    }

    /// 🔴 `ValuesAndComments` なら、`grep -n` が返す**同じ5行**が種別付きで返る。
    /// 偽陽性を消すのではなく、**別枠にする**のがこの旗の意味である。
    #[test]
    fn asking_for_comments_returns_the_same_five_lines_grep_does() {
        let found = parse(WORKFLOW)
            .expect("fixture は読める")
            .search("cancelled()", SearchScope::ValuesAndComments);
        assert_eq!(lines(&found), [4_u32, 29_u32, 30_u32, 33_u32, 46_u32]);
        let kinds: Vec<HitKind> = found.iter().map(Hit::kind).collect();
        assert_eq!(
            kinds,
            [
                HitKind::Comment,
                HitKind::Comment,
                HitKind::Comment,
                HitKind::Value,
                HitKind::Value
            ]
        );
        let pointers: Vec<_> = found.iter().map(|hit| hit.path().pointer()).collect();
        assert_eq!(
            pointers,
            [
                "",
                "/jobs/frontend-check/steps",
                "/jobs/frontend-check/steps",
                "/jobs/frontend-check/steps/3/if",
                "/jobs/e2e/steps/2/if"
            ]
        );
    }

    /// ステップの直前のコメントは、**前のステップの子ではない**。
    /// `tree-sitter-yaml` の木ではそうなる（設計メモ「D-2 実測」）。桁だけを見る規則なら起きない。
    #[test]
    fn a_comment_before_a_step_is_not_attached_to_the_previous_step() {
        let found = parse(WORKFLOW)
            .expect("fixture は読める")
            .search("1) 散文", SearchScope::ValuesAndComments);
        let first = found.first().expect("29 行目のコメントがある");
        assert_eq!(first.line().get(), 29_u32);
        assert_eq!(first.path().pointer(), "/jobs/frontend-check/steps");
        assert_eq!(first.path().label(), None);
    }

    /// 要素の中に書かれたコメントは、値と同じラベルの付いた要素に属する。
    #[test]
    fn a_comment_inside_a_step_carries_the_same_label_as_its_values() {
        let found = parse(WORKFLOW)
            .expect("fixture は読める")
            .search("2) 欠陥", SearchScope::ValuesAndComments);
        let first = found.first().expect("32 行目のコメントがある");
        assert_eq!(first.line().get(), 32_u32);
        assert_eq!(first.path().pointer(), "/jobs/frontend-check/steps/3");
        assert_eq!(
            format!("{}", first.path()),
            "jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\""
        );
    }

    /// ラベルの無い要素は索引だけを出す（D-3）。
    #[test]
    fn an_element_without_a_name_shows_only_its_index() {
        let found = hits(WORKFLOW, "actions/checkout@v4");
        let first = found.first().expect("checkout がある");
        assert_eq!(first.path().label(), None);
        assert_eq!(
            format!("{}", first.path()),
            "jobs.frontend-check.steps[0].uses"
        );
    }

    /// `with:` の下の `name:` は**ラベルではない**（シーケンス要素の直下ではない）。
    #[test]
    fn a_nested_name_is_not_a_label() {
        let found = hits(WORKFLOW, "playwright-report");
        let first = found.first().expect("with の中の name がある");
        assert_eq!(first.path().pointer(), "/jobs/e2e/steps/2/with/name");
        assert_eq!(first.path().label(), Some("Upload Playwright report"));
    }
}
