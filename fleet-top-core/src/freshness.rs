//! 「古い」の基準（今日と、何日で古いと呼ぶか）。

use crate::day::Day;

/// 枝が古いかどうかを決める基準。
///
/// 🔑 **「今日」を値として持つ**のがこの型の役目である。中核は `no_std` で時計に
/// 到達できない（ARC-003）ので、時刻は bin が取って `Day` にして渡す。
/// ⇒ 同じ [`Freshness`] を渡せば、いつ実行しても同じ表が出る。テストが書ける。
/// 🔴 **`Copy` にしない。** この型は `&Freshness` として配り回るもので（[`crate::table::render`]・
/// [`crate::remote_state::RemoteState::stale_branches`]）、`Copy` にすると
/// `clippy::trivially_copy_pass_by_ref` が「参照で渡すな」と言い、公開 API が
/// 値渡しに引きずられる。基準は 1 つで、写しを増やす理由が無い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Freshness {
    today: Day,
    stale_days: u32,
}

impl Freshness {
    /// 今日と、古いと呼ぶまでの日数から作る。
    #[must_use]
    pub fn new(today: Day, stale_days: u32) -> Self {
        Self { today, stale_days }
    }

    /// 基準になる「今日」。
    #[must_use]
    pub fn today(&self) -> Day {
        self.today
    }

    /// 古いと呼ぶまでの日数（`--stale-days`。既定 30）。
    #[must_use]
    pub fn stale_days(&self) -> u32 {
        self.stale_days
    }

    /// 最終コミットがこの日の枝を「古い」と呼ぶか。
    ///
    /// 境界は**超えたら古い**である（`stale_days` 日ちょうどは古くない）。
    /// 🔑 未来の日付（`days_since` が `None`）は古くない。手元の時計がずれているだけで
    /// 枝が古くなるのはおかしいので、**数えないほうに倒す**。
    pub(crate) fn is_stale(&self, last_commit: Day) -> bool {
        self.today
            .days_since(last_commit)
            .is_some_and(|days| days > self.stale_days)
    }
}

#[cfg(test)]
mod tests {
    use super::Freshness;
    use crate::day::Day;

    fn day(text: &str) -> Day {
        Day::parse_iso8601(text).expect("読めるはずである")
    }

    fn freshness(stale_days: u32) -> Freshness {
        Freshness::new(day("2026-09-02"), stale_days)
    }

    #[test]
    fn keeps_what_it_was_given() {
        let found = freshness(30_u32);
        assert_eq!(found.today(), day("2026-09-02"));
        assert_eq!(found.stale_days(), 30_u32);
    }

    /// ちょうど `stale_days` 日は古くない。1 日超えたら古い。
    #[test]
    fn the_boundary_is_strictly_greater() {
        let found = freshness(30_u32);
        assert!(!found.is_stale(day("2026-08-03")));
        assert!(found.is_stale(day("2026-08-02")));
    }

    #[test]
    fn today_is_never_stale() {
        assert!(!freshness(0_u32).is_stale(day("2026-09-02")));
        assert!(freshness(0_u32).is_stale(day("2026-09-01")));
    }

    /// 未来の日付は古くない（手元の時計がずれているだけで枝を古くしない）。
    #[test]
    fn the_future_is_not_stale() {
        assert!(!freshness(0_u32).is_stale(day("2026-09-03")));
    }
}
