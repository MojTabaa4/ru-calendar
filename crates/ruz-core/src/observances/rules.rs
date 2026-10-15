use crate::day::Weekday;

/// When in its calendar's year an observance falls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateRule {
    Fixed {
        month: u8,
        day: u8,
    },
    /// The last day of the month, whatever its length (e.g. the end of Safar).
    LastDayOfMonth {
        month: u8,
    },
    /// The `n`-th `weekday` of the month; `n = -1` is the last one.
    NthWeekday {
        month: u8,
        weekday: Weekday,
        n: i8,
    },
}

impl DateRule {
    pub fn month(&self) -> u8 {
        match *self {
            DateRule::Fixed { month, .. }
            | DateRule::LastDayOfMonth { month }
            | DateRule::NthWeekday { month, .. } => month,
        }
    }

    /// Whether [`DateRule::matches`] looks at `month_len`, so callers can skip computing it.
    pub fn needs_month_len(&self) -> bool {
        matches!(self, DateRule::LastDayOfMonth { .. } | DateRule::NthWeekday { n: ..0, .. })
    }

    pub fn matches(&self, month: u8, day: u8, month_len: u8, weekday: Weekday) -> bool {
        if month != self.month() {
            return false;
        }
        match *self {
            DateRule::Fixed { day: d, .. } => day == d,
            DateRule::LastDayOfMonth { .. } => day == month_len,
            DateRule::NthWeekday { weekday: w, n, .. } => {
                weekday == w
                    && if n > 0 {
                        (day - 1) / 7 + 1 == n as u8
                    } else {
                        (month_len - day) / 7 == (-n - 1) as u8
                    }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_day_follows_month_length() {
        let rule = DateRule::LastDayOfMonth { month: 2 };
        assert!(rule.matches(2, 29, 29, Weekday::Saturday));
        assert!(!rule.matches(2, 29, 30, Weekday::Saturday));
        assert!(rule.matches(2, 30, 30, Weekday::Sunday));
    }

    #[test]
    fn nth_weekday() {
        let second_friday = DateRule::NthWeekday { month: 5, weekday: Weekday::Friday, n: 2 };
        assert!(second_friday.matches(5, 9, 31, Weekday::Friday));
        assert!(!second_friday.matches(5, 2, 31, Weekday::Friday));
        let last_friday = DateRule::NthWeekday { month: 5, weekday: Weekday::Friday, n: -1 };
        assert!(last_friday.matches(5, 30, 31, Weekday::Friday));
        assert!(!last_friday.matches(5, 23, 31, Weekday::Friday));
    }
}
