//! 所属で絞り込むパターン。

use alloc::vec::Vec;

use crate::pattern_segment::PatternSegment;
use crate::scope_path::ScopePath;
use crate::scope_pattern_error::ScopePatternError;
use crate::segment::Segment;

/// 所属で絞り込むパターン（`/jobs/*/steps/*/if`）。
///
/// 形は JSON Pointer（RFC 6901）に合わせる。**所属を出すときと同じ記法で絞れる**ので、
/// 出力を見てから絞り込みを書くときに、記法を翻訳しなくてよい（設計メモ D-1）。
///
/// - `*` は**ちょうど1セグメント**
/// - `**` は**0 個以上のセグメント**
/// - それ以外は、エスケープを解いた**生のキー／索引と完全一致**
///
/// 判定は所属パス**全体**に対する一致である（前方一致ではない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopePattern {
    segments: Vec<PatternSegment>,
}

impl ScopePattern {
    /// パターンを読む。
    ///
    /// # Errors
    ///
    /// 空・先頭が `/` でない・空のセグメントを含む場合は [`ScopePatternError`] を返す。
    /// **黙って直さない**（`jobs/*` を `/jobs/*` と読み替えると、書いた人の意図と
    /// 道具の解釈が静かにずれる）。
    pub fn parse(text: &str) -> Result<Self, ScopePatternError> {
        if text.is_empty() {
            return Err(ScopePatternError::Empty);
        }
        let body = text.strip_prefix('/').ok_or(ScopePatternError::NotRooted)?;
        let mut segments = Vec::new();
        for token in body.split('/') {
            if token.is_empty() {
                return Err(ScopePatternError::EmptySegment);
            }
            segments.push(PatternSegment::read(token));
        }
        Ok(Self { segments })
    }

    /// 所属パス**全体**がこのパターンに一致するか。
    ///
    /// 🔑 ルートに書かれたコメントの所属は空（JSON Pointer の `""`）なので、
    /// `**` だけで始まるパターン（`/**`）にしか当たらない。
    #[must_use]
    pub fn matches(&self, path: &ScopePath) -> bool {
        covers(&self.segments, path.segments())
    }
}

/// パターンの残りと、所属の残りを突き合わせる。
///
/// 🔑 `**` があるので単純な1対1の走査では足りない。**残りの切り口を全部試す**
/// 素直な再帰にしてある。パターンも所属も短い（実測で最長 7 要素）ので、
/// これで足りないほど大きくなることは起きない。
fn covers(pattern: &[PatternSegment], segments: &[Segment]) -> bool {
    let Some((head, rest)) = pattern.split_first() else {
        return segments.is_empty();
    };
    match *head {
        PatternSegment::AnyDepth => any_suffix(rest, segments),
        PatternSegment::Any | PatternSegment::Literal(_) => first_then(head, rest, segments),
    }
}

/// `**` の後ろを、所属の**あらゆる残り**に対して試す（0 個以上に当たる）。
fn any_suffix(pattern: &[PatternSegment], segments: &[Segment]) -> bool {
    (0_usize..=segments.len()).any(|skip| covers(pattern, segments.get(skip..).unwrap_or(&[])))
}

/// 先頭1つを突き合わせ、合えば残りへ進む。
fn first_then(head: &PatternSegment, pattern: &[PatternSegment], segments: &[Segment]) -> bool {
    let Some((first, tail)) = segments.split_first() else {
        return false;
    };
    head.accepts(first) && covers(pattern, tail)
}

#[cfg(test)]
mod tests {
    use super::ScopePattern;
    use crate::scope_path::ScopePath;
    use crate::scope_pattern_error::ScopePatternError;
    use crate::segment::Segment;
    use alloc::borrow::ToOwned;
    use alloc::vec;
    use alloc::vec::Vec;

    /// `/a/b/2` のような JSON Pointer から所属パスを組む（索引は数字のセグメント）。
    fn path(pointer: &str) -> ScopePath {
        let segments: Vec<Segment> = pointer
            .split('/')
            .skip(1_usize)
            .map(|token| match token.parse::<usize>() {
                Ok(index) => Segment::Index { index, label: None },
                Err(_) => Segment::Key(token.to_owned()),
            })
            .collect();
        ScopePath::new(segments)
    }

    fn matches(pattern: &str, pointer: &str) -> bool {
        let Ok(parsed) = ScopePattern::parse(pattern) else {
            return false;
        };
        parsed.matches(&path(pointer))
    }

    // ── 解析 ───────────────────────────────────────────────────────────────

    #[test]
    fn a_pattern_must_be_rooted() {
        assert_eq!(
            ScopePattern::parse("jobs/steps"),
            Err(ScopePatternError::NotRooted)
        );
    }

    #[test]
    fn an_empty_pattern_is_an_error() {
        assert_eq!(ScopePattern::parse(""), Err(ScopePatternError::Empty));
    }

    /// `//` と末尾の `/` は同じ誤り（空のセグメント）である。
    #[test]
    fn an_empty_segment_is_an_error() {
        assert_eq!(
            ScopePattern::parse("/jobs//steps"),
            Err(ScopePatternError::EmptySegment)
        );
        assert_eq!(
            ScopePattern::parse("/jobs/"),
            Err(ScopePatternError::EmptySegment)
        );
        assert_eq!(
            ScopePattern::parse("/"),
            Err(ScopePatternError::EmptySegment)
        );
    }

    // ── 一致 ───────────────────────────────────────────────────────────────

    /// 🔴 前方一致ではない。**全体一致**である。
    #[test]
    fn a_pattern_matches_the_whole_path() {
        assert!(matches("/jobs/e2e/steps", "/jobs/e2e/steps"));
        assert!(!matches("/jobs/e2e", "/jobs/e2e/steps"));
        assert!(!matches("/jobs/e2e/steps/0", "/jobs/e2e/steps"));
    }

    /// `*` は**ちょうど1セグメント**。0 個にも 2 個にも当たらない。
    #[test]
    fn a_single_star_is_exactly_one_segment() {
        assert!(matches("/jobs/*/steps", "/jobs/e2e/steps"));
        assert!(matches("/jobs/*/steps/*/if", "/jobs/e2e/steps/2/if"));
        assert!(!matches("/jobs/*", "/jobs/e2e/steps"));
        assert!(!matches("/jobs/*/steps", "/jobs/steps"));
    }

    /// `**` は**0 個以上**。深さを問わない。
    #[test]
    fn a_double_star_is_zero_or_more_segments() {
        assert!(matches("/jobs/**/if", "/jobs/e2e/steps/2/if"));
        assert!(matches("/jobs/**/if", "/jobs/if"));
        assert!(matches("/**/if", "/if"));
        assert!(matches("/services/**/image", "/services/db/image"));
        assert!(!matches("/jobs/**/if", "/jobs/e2e/steps/2/run"));
    }

    /// 🔴 ルートのコメントの所属は空である。当たるのは `/**` だけ。
    #[test]
    fn only_a_double_star_matches_the_root() {
        let root = ScopePath::new(vec![]);
        let Ok(deep) = ScopePattern::parse("/**") else {
            panic!("読めるはず");
        };
        let Ok(one) = ScopePattern::parse("/*") else {
            panic!("読めるはず");
        };
        assert!(deep.matches(&root));
        assert!(!one.matches(&root));
        assert_eq!(root.pointer(), "");
    }

    /// リテラルは**エスケープを解いた後の生のキー**と比べる。
    #[test]
    fn a_literal_is_compared_after_unescaping() {
        let odd = ScopePath::new(vec![
            Segment::Key("a/b".to_owned()),
            Segment::Key("c~d".to_owned()),
        ]);
        let Ok(pattern) = ScopePattern::parse("/a~1b/c~0d") else {
            panic!("読めるはず");
        };
        assert!(pattern.matches(&odd));
        assert_eq!(odd.pointer(), "/a~1b/c~0d");
    }

    /// 索引は 10 進の文字列として当てる。
    #[test]
    fn an_index_matches_its_decimal_text() {
        assert!(matches("/steps/2", "/steps/2"));
        assert!(!matches("/steps/3", "/steps/2"));
    }
}
