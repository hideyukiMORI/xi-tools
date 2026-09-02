//! 1970-01-01 からの日数。

/// 1970-01-01（UTC）からの日数。
///
/// 🔑 **時刻を持たない**のがこの型の要点である。`fleet-top` が知りたいのは
/// 「その枝は何日前に触られたか」だけで、時分秒は表に出ない。日に落としておけば
/// 中核は時刻を持たずに済み、`no_std` のまま「今日」を**値として**受け取れる（ARC-003）。
///
/// フィールドは非公開で、生成経路は [`Day::from_unix_seconds`] と
/// [`Day::parse_iso8601`] だけである（RS-001 / RS-003）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Day(u32);

/// 1 日の秒数。
const SECONDS_PER_DAY: u64 = 86_400;

/// 受け付ける年の下限（`Day` の原点）。
const MIN_YEAR: u32 = 1970;
/// 受け付ける年の上限（`YYYY` で書ける最後の年）。
const MAX_YEAR: u32 = 9999;

/// `YYYY-MM-DD` の長さ。
const DATE_LENGTH: usize = 10;

/// 各月の日数（平年）。
const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

impl Day {
    /// Unix 時刻（秒・UTC）から作る。
    ///
    /// うるう秒は無いものとして 86,400 で割る（Unix 時刻の定義どおり）。
    /// `u32` に収まらないほど先の時刻は上限で頭打ちにする（巻き戻さない・RS-017）。
    #[must_use]
    pub fn from_unix_seconds(seconds: u64) -> Self {
        let days = seconds / SECONDS_PER_DAY;
        Self(u32::try_from(days).unwrap_or(u32::MAX))
    }

    /// `YYYY-MM-DD` または `YYYY-MM-DDThh:mm:ssZ` を読む。
    ///
    /// 🔑 GitHub GraphQL の `committedDate` / `updatedAt` は後者の形で返る
    /// （実測: `2025-11-09T16:25:19Z`）。時刻部は形だけ検証し、日には影響させない。
    /// **`Z` 以外のタイムゾーンは受けない**——受けると「その日」がどの時計の話か
    /// 分からなくなり、鮮度の判定が場所によって変わる。
    #[must_use]
    pub fn parse_iso8601(text: &str) -> Option<Self> {
        let (date, time) = text.split_at_checked(DATE_LENGTH)?;
        if !(time.is_empty() || is_utc_time(time)) {
            return None;
        }
        let (year, month, day) = split_date(date)?;
        days_from_civil(year, month, day).map(Self)
    }

    /// `earlier` からの経過日数。`self` のほうが古ければ `None`。
    #[must_use]
    pub fn days_since(self, earlier: Self) -> Option<u32> {
        self.0.checked_sub(earlier.0)
    }

    /// 1970-01-01 からの日数。
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

/// `Thh:mm:ssZ` の形かどうか。値は日に影響しないので、範囲だけ見る。
fn is_utc_time(text: &str) -> bool {
    let Some(body) = text
        .strip_prefix('T')
        .and_then(|rest| rest.strip_suffix('Z'))
    else {
        return false;
    };
    let Some((hour, rest)) = body.split_once(':') else {
        return false;
    };
    let Some((minute, second)) = rest.split_once(':') else {
        return false;
    };
    within(hour, 23_u32) && within(minute, 59_u32) && within(second, 60_u32)
}

/// 2 桁の数で、`max` 以下か。
fn within(text: &str, max: u32) -> bool {
    two_digits(text).is_some_and(|value| value <= max)
}

/// `YYYY-MM-DD` を 3 つの数に割る。
fn split_date(text: &str) -> Option<(u32, u32, u32)> {
    let (year, rest) = text.split_once('-')?;
    let (month, day) = rest.split_once('-')?;
    Some((digits(year, 4_usize)?, two_digits(month)?, two_digits(day)?))
}

/// 2 桁の 10 進数。
fn two_digits(text: &str) -> Option<u32> {
    digits(text, 2_usize)
}

/// ちょうど `width` 桁の 10 進数。
///
/// 🔴 `str::parse` に任せない。`+1` や `-1` を通してしまい、`2026-+1-01` が
/// 日付として読めてしまう（符号を受けるのは `parse` の仕様である）。
fn digits(text: &str, width: usize) -> Option<u32> {
    if text.len() != width || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u32>().ok()
}

/// うるう年か（proleptic Gregorian）。
fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4_u32) && (!year.is_multiple_of(100_u32) || year.is_multiple_of(400_u32))
}

/// その年その月の日数。月が 1〜12 でなければ `None`。
fn days_in_month(year: u32, month: u32) -> Option<u32> {
    let index = usize::try_from(month.checked_sub(1_u32)?).ok()?;
    let base = DAYS_IN_MONTH.get(index).copied()?;
    if month == 2_u32 && is_leap_year(year) {
        return Some(29_u32);
    }
    Some(base)
}

/// 暦の日付を 1970-01-01 からの日数に直す。
///
/// Howard Hinnant の `days_from_civil`（proleptic Gregorian の整数アルゴリズム）。
/// 年を 3 月始まりにずらすと、うるう日が年の末尾に来て場合分けが消える。
/// 🔑 年を 1970〜9999 に限っているので、途中の値は全て非負である（`as` も符号も要らない）。
fn days_from_civil(year: u32, month: u32, day: u32) -> Option<u32> {
    if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
        return None;
    }
    if day == 0_u32 || day > days_in_month(year, month)? {
        return None;
    }
    let shifted = if month <= 2_u32 {
        year.checked_sub(1_u32)?
    } else {
        year
    };
    let era = shifted / 400_u32;
    let year_of_era = shifted.checked_sub(era.checked_mul(400_u32)?)?;
    let within_era = day_of_era(year_of_era, month, day)?;
    era.checked_mul(146_097_u32)?
        .checked_add(within_era)?
        .checked_sub(719_468_u32)
}

/// 400 年周期（era）の中で何日目か。0〜146,096。
///
/// 🔑 3 月を年の頭にずらすと、月の長さの並びが 153 日 / 5 か月の等差になり、
/// 月ごとの表を引かずに通日が出る（これが Hinnant のアルゴリズムの要点である）。
fn day_of_era(year_of_era: u32, month: u32, day: u32) -> Option<u32> {
    let month_index = if month > 2_u32 {
        month.checked_sub(3_u32)?
    } else {
        month.checked_add(9_u32)?
    };
    let day_of_year = (153_u32.checked_mul(month_index)?.checked_add(2_u32)? / 5_u32)
        .checked_add(day)?
        .checked_sub(1_u32)?;
    year_of_era
        .checked_mul(365_u32)?
        .checked_add(year_of_era / 4_u32)?
        .checked_sub(year_of_era / 100_u32)?
        .checked_add(day_of_year)
}

#[cfg(test)]
mod tests {
    use super::Day;

    #[test]
    fn unix_seconds_become_whole_days() {
        assert_eq!(Day::from_unix_seconds(0_u64).get(), 0_u32);
        assert_eq!(Day::from_unix_seconds(86_399_u64).get(), 0_u32);
        assert_eq!(Day::from_unix_seconds(86_400_u64).get(), 1_u32);
        assert_eq!(Day::from_unix_seconds(86_401_u64).get(), 1_u32);
    }

    /// `u32` に収まらない時刻は頭打ちにする。巻き戻さない（RS-017）。
    #[test]
    fn far_future_seconds_saturate() {
        assert_eq!(Day::from_unix_seconds(u64::MAX).get(), u32::MAX);
    }

    /// 🔴 既知の日付と照合する（`date -u -d <日付> +%s` を 86400 で割って確認した値）。
    #[test]
    fn matches_known_days() {
        let known = [
            ("1970-01-01", 0_u32),
            ("2000-03-01", 11_017_u32),
            ("2024-02-29", 19_782_u32),
            ("2026-09-02", 20_698_u32),
            ("9999-12-31", 2_932_896_u32),
        ];
        for (text, expected) in known {
            assert_eq!(
                Day::parse_iso8601(text).map(Day::get),
                Some(expected),
                "{text} の日数が合わない"
            );
        }
    }

    #[test]
    fn accepts_the_github_timestamp_form() {
        assert_eq!(
            Day::parse_iso8601("2025-11-09T16:25:19Z").map(Day::get),
            Day::parse_iso8601("2025-11-09").map(Day::get)
        );
        // 日をまたぐ直前の時刻でも、日は変わらない。
        assert_eq!(
            Day::parse_iso8601("2026-09-02T23:59:60Z"),
            Day::parse_iso8601("2026-09-02")
        );
    }

    #[test]
    fn refuses_other_timezones_and_shapes() {
        let refused = [
            "2026-09-02T12:00:00+09:00",
            "2026-09-02T12:00:00",
            "2026-09-02 12:00:00Z",
            "2026-09-02T12:00Z",
            "2026-09-02T123456Z",
            "2026-09-02T24:00:00Z",
            "2026-09-02T12:60:00Z",
            "2026-09-02T12:00:61Z",
            "2026-09-02Textra",
            "2026-09-02T1a:00:00Z",
        ];
        for text in refused {
            assert_eq!(Day::parse_iso8601(text), None, "{text} を受けてしまった");
        }
    }

    #[test]
    fn refuses_impossible_dates() {
        let refused = [
            "2026-02-30",
            "2100-02-29",
            "2026-00-01",
            "2026-13-01",
            "2026-09-00",
            "2026-09-31",
            "1969-12-31",
            "0001-01-01",
        ];
        for text in refused {
            assert_eq!(Day::parse_iso8601(text), None, "{text} を受けてしまった");
        }
    }

    #[test]
    fn accepts_leap_days_only_in_leap_years() {
        assert!(Day::parse_iso8601("2024-02-29").is_some());
        assert!(Day::parse_iso8601("2000-02-29").is_some());
        assert!(Day::parse_iso8601("2100-02-29").is_none());
        assert!(Day::parse_iso8601("2023-02-29").is_none());
    }

    /// 🔴 `str::parse` 任せだと通ってしまう形を拒む。
    #[test]
    fn refuses_signed_and_short_fields() {
        let refused = ["2026-+9-02", "2026-9-02", "202-09-02", "2026/09/02", ""];
        for text in refused {
            assert_eq!(Day::parse_iso8601(text), None, "{text} を受けてしまった");
        }
    }

    /// マルチバイト文字で 10 バイト目を割らない（`split_at` なら panic する位置）。
    #[test]
    fn refuses_a_multibyte_input_without_panicking() {
        assert_eq!(Day::parse_iso8601("2026-09-0あ"), None);
        assert_eq!(Day::parse_iso8601("あいうえお"), None);
    }

    #[test]
    fn days_since_counts_forward_only() {
        let earlier = Day::parse_iso8601("2026-08-30").expect("読める");
        let later = Day::parse_iso8601("2026-09-02").expect("読める");
        assert_eq!(later.days_since(earlier), Some(3_u32));
        assert_eq!(later.days_since(later), Some(0_u32));
        assert_eq!(earlier.days_since(later), None);
    }

    #[test]
    fn days_are_ordered() {
        let earlier = Day::parse_iso8601("2026-08-30").expect("読める");
        let later = Day::parse_iso8601("2026-09-02").expect("読める");
        assert!(earlier < later);
    }
}
