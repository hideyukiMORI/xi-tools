//! ヒットした値が属する場所（所属パス）。

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::segment::Segment;

/// 値が属する場所。**同じ場所を人向けと機械向けの2つの形で言う**（設計メモ D-1）。
///
/// - 機械向け: [`ScopePath::pointer`]（RFC 6901 の JSON Pointer）
/// - 人向け: [`core::fmt::Display`]（`jobs.e2e.steps[2] "Upload Playwright report" .if`）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopePath {
    segments: Vec<Segment>,
}

impl ScopePath {
    /// 要素の並びから作る。
    pub(crate) fn new(segments: Vec<Segment>) -> Self {
        Self { segments }
    }

    /// RFC 6901 の JSON Pointer（`/jobs/e2e/steps/2/if`）。
    ///
    /// キーの `~` は `~0`、`/` は `~1` に退避する。
    /// **ラベルは含めない**（規格に無い情報を規格の中に混ぜない）。
    #[must_use]
    pub fn pointer(&self) -> String {
        let mut out = String::new();
        for segment in &self.segments {
            out.push('/');
            out.push_str(&segment.pointer_token());
        }
        out
    }

    /// 最も内側のシーケンス要素のラベル（その要素の `name` の値）。
    ///
    /// ラベルの無い要素なら `None`。索引だけを出す（設計メモ D-3）。
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| match *segment {
                Segment::Key(_) => None,
                Segment::Index { ref label, .. } => Some(label.as_deref()),
            })
            .flatten()
    }

    /// ラベル表（要素の JSON Pointer → `name` の値）を当てはめた並びを返す。
    ///
    /// 🔑 ラベルは走査中には確定しない。`name:` は `if:` より後に書けるので、
    /// **読み終えてから当てはめる**。そうしないと「`name` が後にある要素だけ
    /// ラベルが落ちる」という、見えにくい取りこぼしが出る。
    pub(crate) fn with_labels(self, labels: &BTreeMap<String, String>) -> Self {
        let mut pointer = String::new();
        let segments = self
            .segments
            .into_iter()
            .map(|segment| {
                pointer.push('/');
                pointer.push_str(&segment.pointer_token());
                let label = labels.get(&pointer).cloned();
                segment.with_label(label)
            })
            .collect();
        Self { segments }
    }
}

impl fmt::Display for ScopePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut after_label = false;
        for (position, segment) in self.segments.iter().enumerate() {
            f.write_str(&segment.render(position == 0_usize, after_label))?;
            after_label = segment.ends_with_label();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ScopePath;
    use crate::segment::Segment;
    use alloc::borrow::ToOwned;
    use alloc::collections::BTreeMap;
    use alloc::format;
    use alloc::vec;

    fn key(name: &str) -> Segment {
        Segment::Key(name.to_owned())
    }

    fn item(index: usize, label: Option<&str>) -> Segment {
        Segment::Index {
            index,
            label: label.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn display_puts_a_space_before_the_key_after_a_label() {
        let path = ScopePath::new(vec![
            key("jobs"),
            key("frontend-check"),
            key("steps"),
            item(3_usize, Some("Audit (fail on high/critical)")),
            key("if"),
        ]);
        assert_eq!(
            format!("{path}"),
            "jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if"
        );
    }

    #[test]
    fn display_without_label_keeps_the_dot() {
        let path = ScopePath::new(vec![
            key("jobs"),
            key("e2e"),
            key("steps"),
            item(0_usize, None),
            key("uses"),
        ]);
        assert_eq!(format!("{path}"), "jobs.e2e.steps[0].uses");
    }

    #[test]
    fn display_quotes_keys_with_odd_characters() {
        let path = ScopePath::new(vec![key("weird key"), key("x")]);
        assert_eq!(format!("{path}"), "\"weird key\".x");
    }

    #[test]
    fn display_escapes_quotes_inside_a_label() {
        let path = ScopePath::new(vec![key("steps"), item(0_usize, Some("a\"b\\c"))]);
        assert_eq!(format!("{path}"), "steps[0] \"a\\\"b\\\\c\"");
    }

    #[test]
    fn pointer_escapes_tilde_and_slash() {
        let path = ScopePath::new(vec![key("a~b"), key("c/d"), item(2_usize, None)]);
        assert_eq!(path.pointer(), "/a~0b/c~1d/2");
    }

    #[test]
    fn pointer_never_contains_the_label() {
        let path = ScopePath::new(vec![key("steps"), item(2_usize, Some("Upload")), key("if")]);
        assert_eq!(path.pointer(), "/steps/2/if");
    }

    #[test]
    fn label_is_the_innermost_one() {
        let path = ScopePath::new(vec![
            key("steps"),
            item(1_usize, Some("outer")),
            key("with"),
            item(0_usize, None),
        ]);
        assert_eq!(path.label(), None);
    }

    #[test]
    fn with_labels_fills_the_matching_element() {
        let mut labels = BTreeMap::new();
        labels.insert("/steps/2".to_owned(), "Upload".to_owned());
        let path =
            ScopePath::new(vec![key("steps"), item(2_usize, None), key("if")]).with_labels(&labels);
        assert_eq!(path.label(), Some("Upload"));
        assert_eq!(format!("{path}"), "steps[2] \"Upload\" .if");
    }
}
