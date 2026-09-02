//! [`crate::query::Query`] が持つ照合の担い手。

use alloc::rc::Rc;

use crate::fixed_string::FixedString;
use crate::matcher::Matcher;

/// 照合の担い手。**閉じた選択肢なので enum で表す**（RS-002）。
///
/// 🔑 中核が知っている照合は固定文字列だけで、それ以外は
/// [`Matching::Custom`] として外から渡される（ADR 0002）。
/// `Box` ではなく `Rc` で持つのは、[`crate::query::Query`] が `Clone` であるためである。
#[derive(Debug, Clone)]
pub(crate) enum Matching {
    /// 固定文字列（既定）。
    Fixed(FixedString),
    /// 外から渡された照合（正規表現など）。
    Custom(Rc<dyn Matcher>),
}

impl Matcher for Matching {
    fn find(&self, text: &str) -> Option<usize> {
        match self {
            Self::Fixed(needle) => needle.find(text),
            Self::Custom(matcher) => matcher.find(text),
        }
    }
}

/// 🔴 **外から渡された照合は「同じ物か」でしか比べられない。**
///
/// [`Matcher`] は関数を持つだけなので、中身の等価性は判定できない。
/// 判定できないものを「等しい」と言わないために、
/// [`Matching::Custom`] どうしは**同じ実体を指すときだけ**等しいとする
/// （複製した [`crate::query::Query`] は等しく、同じ正規表現から別に組んだ 2 つは等しくない）。
impl PartialEq for Matching {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Fixed(left), Self::Fixed(right)) => left == right,
            (Self::Custom(left), Self::Custom(right)) => Rc::ptr_eq(left, right),
            (Self::Fixed(_), Self::Custom(_)) | (Self::Custom(_), Self::Fixed(_)) => false,
        }
    }
}

impl Eq for Matching {}

#[cfg(test)]
mod tests {
    use super::Matching;
    use crate::case_match::CaseMatch;
    use crate::fixed_string::FixedString;
    use crate::matcher::Matcher;
    use alloc::rc::Rc;

    /// 語尾が `x` の行にだけ当たる、テスト用の照合。
    #[derive(Debug)]
    struct LastCharacter;

    impl Matcher for LastCharacter {
        fn find(&self, text: &str) -> Option<usize> {
            text.chars()
                .count()
                .checked_sub(1_usize)
                .filter(|_| text.ends_with('x'))
        }
    }

    fn custom() -> Matching {
        Matching::Custom(Rc::new(LastCharacter))
    }

    #[test]
    fn a_fixed_matching_delegates_to_its_needle() {
        let matching = Matching::Fixed(FixedString::new("x", CaseMatch::Exact));
        assert_eq!(matching.find("あいう x"), Some(4_usize));
        assert_eq!(matching.find("abc"), None);
    }

    #[test]
    fn a_custom_matching_delegates_to_the_given_matcher() {
        assert_eq!(custom().find("あいうx"), Some(3_usize));
        assert_eq!(custom().find("abc"), None);
    }

    /// 🔴 独自の照合は**同じ実体を指すときだけ**等しい。
    #[test]
    fn a_custom_matching_is_equal_only_to_itself() {
        let one = custom();
        assert_eq!(one, one.clone());
        assert_ne!(one, custom(), "別に組んだ照合は等しくない");
    }

    #[test]
    fn a_fixed_matching_is_never_equal_to_a_custom_one() {
        let fixed = Matching::Fixed(FixedString::new("x", CaseMatch::Exact));
        assert_ne!(fixed, custom());
        assert_ne!(custom(), fixed);
        assert_eq!(
            fixed,
            Matching::Fixed(FixedString::new("x", CaseMatch::Exact))
        );
    }
}
