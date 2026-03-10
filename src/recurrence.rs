//! Recurrence rules and patterns for repeating events

use crate::error::Result;
use chrono::{DateTime, Datelike, Offset, TimeZone};
use chrono_tz::Tz;
use rrule::Frequency;

/// Recurrence pattern for events
#[derive(Debug, Clone)]
pub struct Recurrence {
    frequency: Frequency,
    interval: u16,
    count: Option<u32>,
    until: Option<DateTime<Tz>>,
    by_weekday: Option<Vec<rrule::Weekday>>,
}

impl Recurrence {
    /// Create a new recurrence pattern.
    ///
    /// All seven RFC 5545 frequencies are supported: `Secondly`, `Minutely`,
    /// `Hourly`, `Daily`, `Weekly`, `Monthly`, and `Yearly`.
    ///
    /// For the most common cases prefer the typed convenience constructors
    /// ([`daily()`](Self::daily), [`weekly()`](Self::weekly), etc.) which
    /// give compile-time guarantees on the frequency value.
    pub fn new(frequency: Frequency) -> Self {
        Self {
            frequency,
            interval: 1,
            count: None,
            until: None,
            by_weekday: None,
        }
    }

    /// Create a daily recurrence pattern
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    ///
    /// let daily = Recurrence::daily().count(30);
    /// ```
    pub fn daily() -> Self {
        Self::new(Frequency::Daily)
    }

    /// Create a weekly recurrence pattern
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    ///
    /// let weekly = Recurrence::weekly().count(10);
    /// ```
    pub fn weekly() -> Self {
        Self::new(Frequency::Weekly)
    }

    /// Create a monthly recurrence pattern
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    ///
    /// let monthly = Recurrence::monthly().count(12);
    /// ```
    pub fn monthly() -> Self {
        Self::new(Frequency::Monthly)
    }

    /// Create a yearly recurrence pattern
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    ///
    /// let yearly = Recurrence::yearly().count(5);
    /// ```
    pub fn yearly() -> Self {
        Self::new(Frequency::Yearly)
    }

    /// Create an hourly recurrence pattern.
    ///
    /// Uses **"same elapsed time"** semantics: each occurrence is exactly
    /// `interval` hours after the previous one in UTC. During DST transitions
    /// the local-time label may shift (e.g. 1:00 AM EST → 3:00 AM EDT when
    /// clocks spring forward) but the actual interval is always exact.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    ///
    /// // Every 2 hours, 12 times
    /// let schedule = Recurrence::hourly().interval(2).count(12);
    /// ```
    pub fn hourly() -> Self {
        Self::new(Frequency::Hourly)
    }

    /// Create a minutely recurrence pattern.
    ///
    /// Uses **"same elapsed time"** semantics via fixed UTC duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    ///
    /// // Every 15 minutes, 8 times
    /// let schedule = Recurrence::minutely().interval(15).count(8);
    /// ```
    pub fn minutely() -> Self {
        Self::new(Frequency::Minutely)
    }

    /// Create a secondly recurrence pattern.
    ///
    /// Uses **"same elapsed time"** semantics via fixed UTC duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    ///
    /// // Every 30 seconds, 10 times
    /// let schedule = Recurrence::secondly().interval(30).count(10);
    /// ```
    pub fn secondly() -> Self {
        Self::new(Frequency::Secondly)
    }

    /// Set the interval between recurrences
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    ///
    /// // Every 2 weeks
    /// let biweekly = Recurrence::weekly().interval(2).count(10);
    /// ```
    pub fn interval(mut self, interval: u16) -> Self {
        self.interval = interval;
        self
    }

    /// Set the maximum number of occurrences
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    ///
    /// let limited = Recurrence::daily().count(30);
    /// ```
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// Set the end date for recurrence
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::{Recurrence, timezone};
    ///
    /// let tz = timezone::parse_timezone("UTC").unwrap();
    /// let end = timezone::parse_datetime_with_tz("2025-12-31 23:59:59", tz).unwrap();
    /// let limited = Recurrence::daily().until(end);
    /// ```
    pub fn until(mut self, until: DateTime<Tz>) -> Self {
        self.until = Some(until);
        self
    }

    /// Set specific weekdays for weekly recurrence
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    /// use rrule::Weekday;
    ///
    /// // Only on weekdays
    /// let weekdays = Recurrence::weekly()
    ///     .weekdays(vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri])
    ///     .count(20);
    /// ```
    pub fn weekdays(mut self, weekdays: Vec<rrule::Weekday>) -> Self {
        self.by_weekday = Some(weekdays);
        self
    }

    /// Get the frequency of this recurrence
    pub fn frequency(&self) -> Frequency {
        self.frequency
    }

    /// Get the interval of this recurrence
    pub fn get_interval(&self) -> u16 {
        self.interval
    }

    /// Get the count limit of this recurrence
    pub fn get_count(&self) -> Option<u32> {
        self.count
    }

    /// Get the until date of this recurrence
    pub fn get_until(&self) -> Option<DateTime<Tz>> {
        self.until
    }

    /// Build an RRule string for this recurrence
    pub fn to_rrule_string(&self, dtstart: DateTime<Tz>) -> Result<String> {
        let mut rrule_str = format!("FREQ={:?}", self.frequency).to_uppercase();

        if self.interval > 1 {
            rrule_str.push_str(&format!(";INTERVAL={}", self.interval));
        }

        if let Some(count) = self.count {
            rrule_str.push_str(&format!(";COUNT={}", count));
        }

        if let Some(until) = self.until {
            // RFC 5545: UNTIL with Z suffix must be in UTC
            let until_utc = until.with_timezone(&chrono::Utc);
            let until_str = until_utc.format("%Y%m%dT%H%M%SZ").to_string();
            rrule_str.push_str(&format!(";UNTIL={}", until_str));
        }

        if let Some(ref weekdays) = self.by_weekday {
            let days: Vec<String> =
                weekdays.iter().map(|wd| format!("{:?}", wd).to_uppercase()).collect();
            rrule_str.push_str(&format!(";BYDAY={}", days.join(",")));
        }

        Ok(format!("DTSTART:{}\nRRULE:{}", dtstart.format("%Y%m%dT%H%M%S"), rrule_str))
    }

    /// Generate occurrences for this recurrence pattern (eager, allocates Vec)
    /// until the recurrence naturally exhausts via `count`, `until`, or
    /// iterator termination.
    ///
    /// Returns a vector of `DateTime<Tz>` representing each occurrence.
    ///
    /// # Note
    /// For better memory efficiency with large recurrence counts, consider using
    /// the lazy [`occurrences()`](Self::occurrences) method instead. If you want
    /// an eager `Vec` with an explicit hard cap, use
    /// [`generate_occurrences_capped()`](Self::generate_occurrences_capped).
    ///
    /// # Errors
    /// Returns [`crate::error::EventixError::RecurrenceError`] if the recurrence has neither
    /// `count` nor `until` set, since collecting an unbounded iterator would
    /// hang or exhaust memory.
    pub fn generate_occurrences(&self, start: DateTime<Tz>) -> Result<Vec<DateTime<Tz>>> {
        if self.count.is_none() && self.until.is_none() {
            return Err(crate::error::EventixError::RecurrenceError(
                "generate_occurrences() requires a bounded recurrence (set count or until). \
                 Use occurrences() for lazy iteration or generate_occurrences_capped() for \
                 a hard cap."
                    .to_string(),
            ));
        }
        Ok(self.occurrences(start).collect())
    }

    /// Generate occurrences eagerly but stop after `max_occurrences` accepted
    /// items.
    pub fn generate_occurrences_capped(
        &self,
        start: DateTime<Tz>,
        max_occurrences: usize,
    ) -> Result<Vec<DateTime<Tz>>> {
        Ok(self.occurrences(start).take(max_occurrences).collect())
    }

    /// Create a lazy iterator over occurrences of this recurrence pattern.
    ///
    /// Unlike [`generate_occurrences()`](Self::generate_occurrences) and
    /// [`generate_occurrences_capped()`](Self::generate_occurrences_capped),
    /// this method returns an iterator that computes each occurrence on demand,
    /// avoiding upfront memory allocation for large or infinite recurrence
    /// patterns.
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::{Recurrence, timezone};
    ///
    /// let tz = timezone::parse_timezone("UTC").unwrap();
    /// let start = timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();
    ///
    /// let recurrence = Recurrence::daily().count(365);
    ///
    /// // Lazy: only computes what you consume
    /// let first_week: Vec<_> = recurrence.occurrences(start).take(7).collect();
    /// assert_eq!(first_week.len(), 7);
    ///
    /// // Skip directly to a future occurrence without computing all intermediate dates
    /// let tenth = recurrence.occurrences(start).nth(9);
    /// assert!(tenth.is_some());
    /// ```
    pub fn occurrences(&self, start: DateTime<Tz>) -> OccurrenceIterator {
        OccurrenceIterator::new(self.clone(), start)
    }
}

/// Advance a datetime by the given frequency and interval.
///
/// Shared helper used by the eager generation helpers and the lazy
/// `OccurrenceIterator`.
///
/// Resolve a `NaiveDateTime` to a timezone-aware `DateTime<Tz>`, handling
/// DST transitions:
///
/// - **Normal / fall-back (ambiguous)**: picks the earlier of two candidates
/// - **Spring-forward (gap)**: the local time doesn't exist; applies the
///   pre-gap UTC offset so the resulting wall-clock time shifts forward by
///   exactly the gap size (e.g. 2:30 AM EST → 3:30 AM EDT), matching
///   Google Calendar / RFC 5545 behaviour
fn resolve_local(tz: Tz, naive: chrono::NaiveDateTime) -> Option<DateTime<Tz>> {
    if let Some(dt) = tz.from_local_datetime(&naive).earliest() {
        return Some(dt);
    }
    // DST gap: the local time doesn't exist.  Determine the UTC offset
    // that was in effect just before the gap by resolving a time one day
    // earlier (guaranteed to exist outside the gap).  Converting the
    // nonexistent local time with that offset naturally lands on the
    // correct post-transition wall-clock time.
    let day_before = naive - chrono::Duration::days(1);
    let pre_gap_dt = tz.from_local_datetime(&day_before).earliest()?;
    let pre_offset = pre_gap_dt.offset().fix();
    let utc_naive = naive - pre_offset;
    Some(chrono::Utc.from_utc_datetime(&utc_naive).with_timezone(&tz))
}

/// Advance a `DateTime<Tz>` by one recurrence step.
///
/// `intended_time` is the original start's wall-clock time. For
/// calendar-aligned frequencies (`Daily`–`Yearly`) it prevents wall-clock
/// drift after DST gap resolution (e.g. 2:30 AM shifted to 3:30 AM on
/// spring-forward day returns to 2:30 AM the following day). Sub-daily
/// frequencies ignore this parameter and always advance from `current`
/// using a fixed UTC duration.
///
/// ## All seven RFC 5545 frequencies are supported
///
/// **Sub-daily** (advance by fixed UTC duration — DST-transparent):
/// - [`Frequency::Secondly`] — adds `interval` seconds
/// - [`Frequency::Minutely`] — adds `interval` minutes
/// - [`Frequency::Hourly`]   — adds `interval` hours
///
/// **Calendar-aligned** (local-date arithmetic + DST resolution):
/// - [`Frequency::Daily`]   — adds `interval` calendar days
/// - [`Frequency::Weekly`]  — adds `interval × 7` calendar days
/// - [`Frequency::Monthly`] — adds `interval` months, clamping to the last
///   valid day (e.g. Jan 31 → Feb 28)
/// - [`Frequency::Yearly`]  — adds `interval` years, clamping for leap days
///   (e.g. Feb 29 → Feb 28 in non-leap years)
fn advance_by_frequency(
    current: DateTime<Tz>,
    frequency: Frequency,
    interval: u16,
    intended_time: chrono::NaiveTime,
) -> Option<DateTime<Tz>> {
    if interval == 0 {
        return None;
    }
    let tz = current.timezone();
    match frequency {
        Frequency::Daily => {
            let new_date = current.date_naive() + chrono::Days::new(interval as u64);
            let naive = chrono::NaiveDateTime::new(new_date, intended_time);
            resolve_local(tz, naive)
        }
        Frequency::Weekly => {
            let new_date = current.date_naive() + chrono::Days::new(interval as u64 * 7);
            let naive = chrono::NaiveDateTime::new(new_date, intended_time);
            resolve_local(tz, naive)
        }
        Frequency::Monthly => {
            let months_to_add = interval as i32;
            let mut new_month = current.month() as i32 + months_to_add;
            let mut new_year = current.year();
            while new_month > 12 {
                new_month -= 12;
                new_year += 1;
            }
            let date = clamp_day_to_month(new_year, new_month as u32, current.day())?;
            let naive = chrono::NaiveDateTime::new(date, intended_time);
            resolve_local(tz, naive)
        }
        Frequency::Yearly => {
            let new_year = current.year() + interval as i32;
            let date = clamp_day_to_month(new_year, current.month(), current.day())?;
            let naive = chrono::NaiveDateTime::new(date, intended_time);
            resolve_local(tz, naive)
        }
        // Sub-daily: advance by a fixed UTC duration ("same elapsed time"
        // semantics, not "same local wall-clock slot").  Adding
        // chrono::Duration to a DateTime<Tz> always goes through UTC,
        // so spring-forward / fall-back transitions are handled transparently
        // without any local-time lookup.
        Frequency::Hourly => Some(current + chrono::Duration::hours(interval as i64)),
        Frequency::Minutely => Some(current + chrono::Duration::minutes(interval as i64)),
        Frequency::Secondly => Some(current + chrono::Duration::seconds(interval as i64)),
    }
}

/// Build a `NaiveDate` for `(year, month, day)`, clamping `day` downward
/// to the last valid day of the month when the original day doesn't exist
/// (e.g. day 31 in a 30-day month, or day 29 in non-leap February).
fn clamp_day_to_month(year: i32, month: u32, day: u32) -> Option<chrono::NaiveDate> {
    // Try the original day first (fast path)
    if let Some(d) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
        return Some(d);
    }
    // Clamp: walk backward from day-1 to 28 (always valid)
    let mut d = day.min(31);
    while d > 28 {
        d -= 1;
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, d) {
            return Some(date);
        }
    }
    // 28 is always valid for months 1-12
    chrono::NaiveDate::from_ymd_opt(year, month, 28)
}

/// For Weekly frequency with specific weekdays: advance to the next matching
/// weekday. Within the current week period, steps forward day-by-day.
/// When no more matching weekdays remain in this week, jumps by
/// `interval` weeks to reach the next week period and finds the first
/// matching weekday there.
fn advance_weekly_weekday(
    current: DateTime<Tz>,
    interval: u16,
    weekdays: &[chrono::Weekday],
    intended_time: chrono::NaiveTime,
) -> Option<DateTime<Tz>> {
    // Match the zero-interval guard in advance_by_frequency(): no further
    // occurrences when interval == 0.
    if interval == 0 {
        return None;
    }

    let tz = current.timezone();
    let date = current.date_naive();
    let current_dow = date.weekday().num_days_from_monday(); // 0=Mon..6=Sun

    // Try remaining days in the current calendar week (Mon-Sun).
    // Only consider days strictly after the current weekday within this week.
    for day_offset in 1u64..(7 - current_dow as u64) {
        let candidate = date + chrono::Days::new(day_offset);
        if weekdays.contains(&candidate.weekday()) {
            let naive = chrono::NaiveDateTime::new(candidate, intended_time);
            return resolve_local(tz, naive);
        }
    }

    // No more matching weekdays this week — jump to the next week period.
    // Find the Monday of the current week, then advance by interval weeks.
    let week_start = date - chrono::Days::new(current_dow as u64);
    let next_week_start = week_start + chrono::Days::new(interval as u64 * 7);

    for day_offset in 0u64..7 {
        let candidate = next_week_start + chrono::Days::new(day_offset);
        if weekdays.contains(&candidate.weekday()) {
            let naive = chrono::NaiveDateTime::new(candidate, intended_time);
            return resolve_local(tz, naive);
        }
    }

    None
}

/// A lazy iterator over recurrence occurrences.
///
/// Created by [`Recurrence::occurrences()`]. This iterator computes each
/// occurrence on demand, making it memory-efficient for large or infinite
/// recurrence patterns.
///
/// # Examples
///
/// ```
/// use eventix::{Recurrence, timezone};
///
/// let tz = timezone::parse_timezone("UTC").unwrap();
/// let start = timezone::parse_datetime_with_tz("2025-06-01 10:00:00", tz).unwrap();
///
/// // Daily recurrence for 30 days
/// let daily = Recurrence::daily().count(30);
///
/// // Iterate lazily - computes dates as needed
/// for occurrence in daily.occurrences(start).take(5) {
///     println!("Occurrence: {}", occurrence);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OccurrenceIterator {
    recurrence: Recurrence,
    current: DateTime<Tz>,
    intended_time: chrono::NaiveTime,
    count: u32,
    exhausted: bool,
}

impl OccurrenceIterator {
    /// Create a new occurrence iterator
    fn new(recurrence: Recurrence, start: DateTime<Tz>) -> Self {
        Self {
            recurrence,
            intended_time: start.time(),
            current: start,
            count: 0,
            exhausted: false,
        }
    }

    /// Check if the iterator is exhausted
    fn is_exhausted(&self) -> bool {
        if self.exhausted {
            return true;
        }

        // Check count limit
        if let Some(max_count) = self.recurrence.count {
            if self.count >= max_count {
                return true;
            }
        }

        // Check until date
        if let Some(until) = self.recurrence.until {
            if self.current > until {
                return true;
            }
        }

        false
    }

    /// Compute the next occurrence date
    fn compute_next(&self) -> Option<DateTime<Tz>> {
        advance_by_frequency(
            self.current,
            self.recurrence.frequency,
            self.recurrence.interval,
            self.intended_time,
        )
    }
}

impl Iterator for OccurrenceIterator {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.is_exhausted() {
                return None;
            }

            // Capture current value
            let result = self.current;

            // Weekly + weekdays: use intra-week expansion
            if self.recurrence.frequency == Frequency::Weekly {
                if let Some(ref weekdays) = self.recurrence.by_weekday {
                    match advance_weekly_weekday(
                        result,
                        self.recurrence.interval,
                        weekdays,
                        self.intended_time,
                    ) {
                        Some(next) => self.current = next,
                        None => self.exhausted = true,
                    }
                    if weekdays.contains(&result.weekday()) {
                        self.count += 1;
                        return Some(result);
                    }
                    continue;
                }
            }

            // Compute next occurrence for future calls
            match self.compute_next() {
                Some(next) => self.current = next,
                None => self.exhausted = true,
            }

            // Skip if weekday filter is set and this day doesn't match
            // (for Daily and other frequencies, not Weekly which is handled above)
            if let Some(ref weekdays) = self.recurrence.by_weekday {
                if !weekdays.contains(&result.weekday()) {
                    continue;
                }
            }

            // Only count emitted (non-skipped) occurrences
            self.count += 1;

            return Some(result);
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(max_count) = self.recurrence.count {
            let remaining = max_count.saturating_sub(self.count) as usize;
            (0, Some(remaining))
        } else if self.recurrence.until.is_some() {
            // Cannot determine exact size without computing
            (0, None)
        } else {
            // Infinite recurrence
            (0, None)
        }
    }
}

/// Filter for skipping certain dates (e.g., weekends, holidays)
#[derive(Debug, Clone)]
pub struct RecurrenceFilter {
    skip_weekends: bool,
    skip_dates: Vec<DateTime<Tz>>,
}

impl RecurrenceFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self {
            skip_weekends: false,
            skip_dates: Vec::new(),
        }
    }

    /// Enable skipping weekends (Saturday and Sunday)
    pub fn skip_weekends(mut self, skip: bool) -> Self {
        self.skip_weekends = skip;
        self
    }

    /// Add specific dates to skip
    pub fn skip_dates(mut self, dates: Vec<DateTime<Tz>>) -> Self {
        self.skip_dates.extend(dates);
        self
    }

    /// Check if a date should be skipped
    pub fn should_skip(&self, date: &DateTime<Tz>) -> bool {
        // Check if it's a weekend
        if self.skip_weekends {
            let weekday = date.weekday();
            if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
                return true;
            }
        }

        // Check if it's in the skip list
        self.skip_dates
            .iter()
            .any(|skip_date| skip_date.date_naive() == date.date_naive())
    }

    /// Filter a list of occurrences
    pub fn filter_occurrences(&self, occurrences: Vec<DateTime<Tz>>) -> Vec<DateTime<Tz>> {
        occurrences.into_iter().filter(|dt| !self.should_skip(dt)).collect()
    }
}

impl Default for RecurrenceFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::timezone::parse_timezone;
    use chrono::Timelike;

    #[test]
    fn test_daily_recurrence() {
        let recurrence = Recurrence::daily().count(5);
        assert_eq!(recurrence.frequency(), Frequency::Daily);
        assert_eq!(recurrence.get_count(), Some(5));
    }

    #[test]
    fn test_weekly_recurrence() {
        let recurrence = Recurrence::weekly().interval(2).count(10);
        assert_eq!(recurrence.frequency(), Frequency::Weekly);
        assert_eq!(recurrence.get_interval(), 2);
        assert_eq!(recurrence.get_count(), Some(10));
    }

    #[test]
    fn test_recurrence_filter_weekends() {
        let filter = RecurrenceFilter::new().skip_weekends(true);

        let tz = parse_timezone("UTC").unwrap();
        let saturday = crate::timezone::parse_datetime_with_tz("2025-11-01 10:00:00", tz).unwrap(); // Saturday
        let monday = crate::timezone::parse_datetime_with_tz("2025-11-03 10:00:00", tz).unwrap(); // Monday

        assert!(filter.should_skip(&saturday));
        assert!(!filter.should_skip(&monday));
    }

    #[test]
    fn test_lazy_iterator_equivalence() {
        // Lazy iterator should produce same results as eager method
        let recurrence = Recurrence::daily().count(10);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let eager: Vec<_> = recurrence.generate_occurrences(start).unwrap();
        let lazy: Vec<_> = recurrence.occurrences(start).collect();

        assert_eq!(eager.len(), lazy.len());
        for (e, l) in eager.iter().zip(lazy.iter()) {
            assert_eq!(e, l);
        }
    }

    #[test]
    fn test_lazy_iterator_take() {
        // Can take fewer items than the limit
        let recurrence = Recurrence::daily().count(100);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-03-15 14:00:00", tz).unwrap();

        let first_5: Vec<_> = recurrence.occurrences(start).take(5).collect();
        assert_eq!(first_5.len(), 5);
        assert_eq!(first_5[0], start);
    }

    #[test]
    fn test_lazy_iterator_nth() {
        // Can skip directly to nth occurrence
        let recurrence = Recurrence::weekly().count(52);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 12:00:00", tz).unwrap();

        let tenth = recurrence.occurrences(start).nth(9);
        assert!(tenth.is_some());

        // Verify it's 9 weeks later (nth(9) = 10th occurrence, which is start + 9 intervals)
        let expected = start + chrono::Duration::weeks(9);
        assert_eq!(tenth.unwrap(), expected);
    }

    #[test]
    fn test_lazy_iterator_size_hint() {
        let recurrence = Recurrence::daily().count(30);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-06-01 08:00:00", tz).unwrap();

        let iter = recurrence.occurrences(start);
        let (min, max) = iter.size_hint();
        assert_eq!(min, 0);
        assert_eq!(max, Some(30));
    }

    #[test]
    fn test_lazy_iterator_monthly() {
        let recurrence = Recurrence::monthly().count(6);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-15 10:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 6);

        // Check months are Jan, Feb, Mar, Apr, May, Jun
        for (i, occ) in occurrences.iter().enumerate() {
            assert_eq!(occ.month(), (1 + i) as u32);
        }
    }

    #[test]
    fn test_lazy_iterator_yearly() {
        let recurrence = Recurrence::yearly().count(5);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-07-04 00:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 5);

        // Check years are 2025-2029
        for (i, occ) in occurrences.iter().enumerate() {
            assert_eq!(occ.year(), 2025 + i as i32);
        }
    }

    #[test]
    fn test_lazy_iterator_until() {
        // Test the `until` exhaustion path in is_exhausted()
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();
        let end = crate::timezone::parse_datetime_with_tz("2025-01-05 09:00:00", tz).unwrap();

        let recurrence = Recurrence::daily().until(end);
        let occurrences: Vec<_> = recurrence.occurrences(start).collect();

        // Should include Jan 1-5 (5 days)
        assert_eq!(occurrences.len(), 5);
        assert_eq!(occurrences.last().unwrap(), &end);
    }

    #[test]
    fn test_lazy_iterator_size_hint_until() {
        // size_hint with `until` returns (0, None) since exact count is unknown
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();
        let end = crate::timezone::parse_datetime_with_tz("2025-12-31 09:00:00", tz).unwrap();

        let recurrence = Recurrence::daily().until(end);
        let iter = recurrence.occurrences(start);
        let (min, max) = iter.size_hint();
        assert_eq!(min, 0);
        assert_eq!(max, None);
    }

    #[test]
    fn test_lazy_iterator_size_hint_infinite() {
        // size_hint with no count and no until returns (0, None)
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let recurrence = Recurrence::daily();
        let iter = recurrence.occurrences(start);
        let (min, max) = iter.size_hint();
        assert_eq!(min, 0);
        assert_eq!(max, None);
    }

    #[test]
    fn test_lazy_iterator_with_interval() {
        // Test bi-weekly via lazy iterator
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 10:00:00", tz).unwrap();

        let recurrence = Recurrence::weekly().interval(2).count(4);
        let occurrences: Vec<_> = recurrence.occurrences(start).collect();

        assert_eq!(occurrences.len(), 4);
        // Each occurrence should be 2 weeks apart
        for i in 1..occurrences.len() {
            let diff = occurrences[i] - occurrences[i - 1];
            assert_eq!(diff, chrono::Duration::weeks(2));
        }
    }

    #[test]
    fn test_monthly_day_clamping() {
        // Jan 31 → Feb 28, Mar 28, Apr 28 ... (clamps to last valid day)
        let recurrence = Recurrence::monthly().count(4);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-31 12:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 4);
        assert_eq!(occurrences[0].day(), 31); // Jan 31
        assert_eq!(occurrences[1].day(), 28); // Feb 28 (2025 is not a leap year)
        assert_eq!(occurrences[1].month(), 2);
        // Subsequent months clamp from 28 (the new current day)
        assert_eq!(occurrences[2].month(), 3);
        assert_eq!(occurrences[3].month(), 4);
    }

    #[test]
    fn test_yearly_leap_day_clamping() {
        // Feb 29 in a leap year → Feb 28 in non-leap years
        let recurrence = Recurrence::yearly().count(3);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2024-02-29 08:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 3);
        assert_eq!(occurrences[0].day(), 29); // 2024 leap year
        assert_eq!(occurrences[1].day(), 28); // 2025 not a leap year
        assert_eq!(occurrences[1].year(), 2025);
        assert_eq!(occurrences[2].day(), 28); // 2026 not a leap year
    }

    #[test]
    fn test_zero_interval_does_not_loop() {
        // interval(0) should cause the iterator to terminate immediately
        let recurrence = Recurrence::daily().interval(0).count(10);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        // Lazy: should yield just the start, then stop (advance returns None)
        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0], start);

        // Eager: same behavior
        let eager = recurrence.generate_occurrences(start).unwrap();
        assert_eq!(eager.len(), 1);
    }

    #[test]
    fn test_zero_interval_weekly_weekdays_does_not_loop() {
        // interval(0) on weekly + weekdays should terminate immediately,
        // consistent with advance_by_frequency() returning None for interval==0.
        let recurrence = Recurrence::weekly()
            .interval(0)
            .weekdays(vec![chrono::Weekday::Mon, chrono::Weekday::Wed])
            .count(5);
        let tz = parse_timezone("UTC").unwrap();
        // Start on a Monday
        let start = crate::timezone::parse_datetime_with_tz("2025-01-06 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        // interval(0) yields only the start (if on a valid weekday), then stops
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0], start);
    }

    #[test]
    fn test_weekdays_filter_lazy() {
        use rrule::Weekday;
        // Daily recurrence on Mon/Wed/Fri with count=14
        // count(14) means 14 emitted (matching) occurrences, not 14 scanned slots
        // Start on a Monday (2025-01-06)
        let recurrence = Recurrence::daily()
            .weekdays(vec![Weekday::Mon, Weekday::Wed, Weekday::Fri])
            .count(14);

        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-06 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();

        // All results should be Mon, Wed, or Fri
        for occ in &occurrences {
            let wd = occ.weekday();
            assert!(
                wd == Weekday::Mon || wd == Weekday::Wed || wd == Weekday::Fri,
                "unexpected weekday: {:?}",
                wd
            );
        }
        // count(14) emits exactly 14 matching weekdays
        assert_eq!(occurrences.len(), 14);
    }

    #[test]
    fn test_weekdays_filter_eager() {
        use rrule::Weekday;
        let recurrence = Recurrence::daily()
            .weekdays(vec![Weekday::Mon, Weekday::Wed, Weekday::Fri])
            .count(14);

        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-06 09:00:00", tz).unwrap();

        let eager = recurrence.generate_occurrences(start).unwrap();
        let lazy: Vec<_> = recurrence.occurrences(start).collect();

        assert_eq!(eager.len(), lazy.len());
        for (e, l) in eager.iter().zip(lazy.iter()) {
            assert_eq!(e, l);
        }
    }

    #[test]
    fn test_weekly_weekdays_expansion_lazy() {
        use rrule::Weekday;
        // Weekly recurrence on Mon/Wed should emit BOTH Mon and Wed each week
        let recurrence = Recurrence::weekly().weekdays(vec![Weekday::Mon, Weekday::Wed]).count(6);

        let tz = parse_timezone("UTC").unwrap();
        // 2025-01-06 is a Monday
        let start = crate::timezone::parse_datetime_with_tz("2025-01-06 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 6);

        // Should be Mon6, Wed8, Mon13, Wed15, Mon20, Wed22
        assert_eq!(occurrences[0].day(), 6); // Mon
        assert_eq!(occurrences[1].day(), 8); // Wed
        assert_eq!(occurrences[2].day(), 13); // Mon
        assert_eq!(occurrences[3].day(), 15); // Wed
        assert_eq!(occurrences[4].day(), 20); // Mon
        assert_eq!(occurrences[5].day(), 22); // Wed

        for occ in &occurrences {
            let wd = occ.weekday();
            assert!(wd == Weekday::Mon || wd == Weekday::Wed);
        }
    }

    #[test]
    fn test_weekly_weekdays_expansion_eager() {
        use rrule::Weekday;
        // Eager path should match lazy for weekly + weekdays
        let recurrence = Recurrence::weekly().weekdays(vec![Weekday::Mon, Weekday::Wed]).count(6);

        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-06 09:00:00", tz).unwrap();

        let eager = recurrence.generate_occurrences(start).unwrap();
        let lazy: Vec<_> = recurrence.occurrences(start).collect();

        assert_eq!(eager.len(), lazy.len());
        for (e, l) in eager.iter().zip(lazy.iter()) {
            assert_eq!(e, l);
        }
    }

    #[test]
    fn test_weekly_weekdays_biweekly() {
        use rrule::Weekday;
        // Every 2 weeks, on Tue/Thu
        let recurrence = Recurrence::weekly()
            .interval(2)
            .weekdays(vec![Weekday::Tue, Weekday::Thu])
            .count(4);

        let tz = parse_timezone("UTC").unwrap();
        // 2025-01-07 is a Tuesday
        let start = crate::timezone::parse_datetime_with_tz("2025-01-07 10:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 4);

        // Week 1: Tue Jan 7, Thu Jan 9
        assert_eq!(occurrences[0].day(), 7);
        assert_eq!(occurrences[1].day(), 9);
        // Week 3 (skip week 2): Tue Jan 21, Thu Jan 23
        assert_eq!(occurrences[2].day(), 21);
        assert_eq!(occurrences[3].day(), 23);
    }

    #[test]
    fn test_weekly_weekdays_start_not_in_weekdays() {
        use rrule::Weekday;
        // Start on a Tuesday but only want Mon/Fri
        let recurrence = Recurrence::weekly().weekdays(vec![Weekday::Mon, Weekday::Fri]).count(4);

        let tz = parse_timezone("UTC").unwrap();
        // 2025-01-07 is a Tuesday
        let start = crate::timezone::parse_datetime_with_tz("2025-01-07 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 4);

        // Tue is skipped, first match is Fri Jan 10, then Mon Jan 13, Fri Jan 17, Mon Jan 20
        for occ in &occurrences {
            let wd = occ.weekday();
            assert!(wd == Weekday::Mon || wd == Weekday::Fri);
        }
    }

    #[test]
    fn test_dst_spring_forward_daily() {
        // US spring-forward: 2025-03-09 2:00 AM → 3:00 AM in America/New_York
        // A daily recurrence at 2:30 AM should survive the DST gap
        let recurrence = Recurrence::daily().count(3);
        let tz = parse_timezone("America/New_York").unwrap();
        // March 8 at 2:30 AM exists
        let start = crate::timezone::parse_datetime_with_tz("2025-03-08 02:30:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        // Should not terminate — all 3 occurrences emitted
        assert_eq!(occurrences.len(), 3);
        // March 8, 9, 10
        assert_eq!(occurrences[0].day(), 8);
        assert_eq!(occurrences[1].day(), 9); // DST gap day — resolved to post-gap time
        assert_eq!(occurrences[2].day(), 10);
        // March 9: 2:30 AM EST doesn't exist, should resolve to 3:30 AM EDT
        // (pre-gap offset UTC-5 applied to 02:30 → 07:30 UTC → 03:30 EDT)
        assert_eq!(occurrences[1].hour(), 3);
        assert_eq!(occurrences[1].minute(), 30);
        // March 10: back to normal 2:30 AM EDT
        assert_eq!(occurrences[2].hour(), 2);
        assert_eq!(occurrences[2].minute(), 30);
    }

    #[test]
    fn test_dst_spring_forward_weekly() {
        // Weekly recurrence crossing DST spring-forward
        let recurrence = Recurrence::weekly().count(3);
        let tz = parse_timezone("America/New_York").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-03-02 02:30:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 3);
        // Mar 2, Mar 9 (DST day), Mar 16
        assert_eq!(occurrences[0].day(), 2);
        assert_eq!(occurrences[1].day(), 9);
        assert_eq!(occurrences[2].day(), 16);
        // Mar 9 should resolve to 3:30 AM EDT (same pre-gap offset logic)
        assert_eq!(occurrences[1].hour(), 3);
        assert_eq!(occurrences[1].minute(), 30);
        // Mar 16 is post-DST, should be 2:30 AM EDT
        assert_eq!(occurrences[2].hour(), 2);
    }

    #[test]
    fn test_dst_fall_back_daily() {
        // US fall-back: 2025-11-02 2:00 AM → 1:00 AM in America/New_York
        // 1:30 AM is ambiguous (exists in both EDT and EST)
        // resolve_local picks .earliest() which is the EDT version
        let recurrence = Recurrence::daily().count(3);
        let tz = parse_timezone("America/New_York").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-11-01 01:30:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 3);
        assert_eq!(occurrences[0].day(), 1);
        assert_eq!(occurrences[1].day(), 2); // ambiguous day
        assert_eq!(occurrences[2].day(), 3);
        // All should be at 1:30 AM wall-clock
        for occ in &occurrences {
            assert_eq!(occ.hour(), 1);
            assert_eq!(occ.minute(), 30);
        }
    }

    #[test]
    fn test_dst_spring_forward_eager_matches_lazy() {
        // Verify eager and lazy paths produce identical results across DST
        let recurrence = Recurrence::daily().count(5);
        let tz = parse_timezone("America/New_York").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-03-07 02:30:00", tz).unwrap();

        let eager = recurrence.generate_occurrences(start).unwrap();
        let lazy: Vec<_> = recurrence.occurrences(start).collect();

        assert_eq!(eager.len(), lazy.len());
        for (e, l) in eager.iter().zip(lazy.iter()) {
            assert_eq!(e, l);
        }
    }

    #[test]
    fn test_hourly_recurrence() {
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-06-01 08:00:00", tz).unwrap();

        let occs: Vec<_> = Recurrence::hourly().count(4).occurrences(start).collect();
        assert_eq!(occs.len(), 4);
        // Each occurrence 1 hour apart
        for i in 1..occs.len() {
            assert_eq!(occs[i] - occs[i - 1], chrono::Duration::hours(1));
        }
    }

    #[test]
    fn test_minutely_recurrence() {
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-06-01 09:00:00", tz).unwrap();

        let occs: Vec<_> =
            Recurrence::minutely().interval(15).count(5).occurrences(start).collect();
        assert_eq!(occs.len(), 5);
        for i in 1..occs.len() {
            assert_eq!(occs[i] - occs[i - 1], chrono::Duration::minutes(15));
        }
    }

    #[test]
    fn test_secondly_recurrence() {
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-06-01 10:00:00", tz).unwrap();

        let occs: Vec<_> =
            Recurrence::secondly().interval(30).count(6).occurrences(start).collect();
        assert_eq!(occs.len(), 6);
        for i in 1..occs.len() {
            assert_eq!(occs[i] - occs[i - 1], chrono::Duration::seconds(30));
        }
    }

    #[test]
    fn test_hourly_across_dst_spring_forward() {
        // US spring-forward: 2025-03-09 2:00 AM → 3:00 AM in America/New_York
        // An hourly series starting at 1:00 AM should smoothly cross the gap
        // (1:00 → 2:00 doesn't exist locally but is valid UTC → shows as 3:00 EDT)
        let tz = parse_timezone("America/New_York").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-03-09 01:00:00", tz).unwrap();

        let occs: Vec<_> = Recurrence::hourly().count(4).occurrences(start).collect();
        assert_eq!(occs.len(), 4);
        // Intervals must be exactly 1 hour apart in wall-clock time
        // (UTC duration arithmetic is always exact)
        for i in 1..occs.len() {
            assert_eq!(occs[i] - occs[i - 1], chrono::Duration::hours(1));
        }
        // The "2:00 AM" slot is skipped by the clocks — next valid hour is 3:00 AM EDT
        assert_eq!(occs[1].hour(), 3);
    }

    #[test]
    fn test_subdaily_new_constructor() {
        // Recurrence::new() accepts all RFC 5545 frequencies
        let _ = Recurrence::new(Frequency::Hourly);
        let _ = Recurrence::new(Frequency::Minutely);
        let _ = Recurrence::new(Frequency::Secondly);
    }

    #[test]
    fn test_generate_occurrences_capped() {
        let recurrence = Recurrence::daily().count(30);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let capped = recurrence.generate_occurrences_capped(start, 5).unwrap();
        assert_eq!(capped.len(), 5);
        assert_eq!(capped[0], start);
    }

    #[test]
    fn test_until_rrule_string_uses_utc() {
        let tz = parse_timezone("America/New_York").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-06-01 10:00:00", tz).unwrap();
        let until = crate::timezone::parse_datetime_with_tz("2025-06-30 10:00:00", tz).unwrap();

        let recurrence = Recurrence::daily().until(until);
        let rrule_str = recurrence.to_rrule_string(start).unwrap();

        // UNTIL must be in UTC (EDT = UTC-4, so 10:00 EDT = 14:00 UTC)
        assert!(
            rrule_str.contains("UNTIL=20250630T140000Z"),
            "UNTIL should be converted to UTC, got: {}",
            rrule_str
        );
    }

    #[test]
    fn test_generate_occurrences_rejects_unbounded() {
        let recurrence = Recurrence::daily(); // no count, no until
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let result = recurrence.generate_occurrences(start);
        assert!(result.is_err(), "unbounded recurrence should be rejected");
    }
}
