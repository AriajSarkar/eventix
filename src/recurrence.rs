//! Recurrence rules and patterns for repeating events

use crate::error::Result;
use chrono::{DateTime, Datelike, TimeZone};
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
    /// Create a new recurrence pattern
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
            let until_str = until.format("%Y%m%dT%H%M%SZ").to_string();
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
    ///
    /// Returns a vector of `DateTime<Tz>` representing each occurrence.
    ///
    /// # Note
    /// For better memory efficiency with large recurrence counts, consider using
    /// the lazy [`occurrences()`](Self::occurrences) method instead.
    pub fn generate_occurrences(
        &self,
        start: DateTime<Tz>,
        max_occurrences: usize,
    ) -> Result<Vec<DateTime<Tz>>> {
        // Simplified recurrence generation without using rrule library for now
        // This is a basic implementation that handles common cases

        let mut occurrences = Vec::new();
        let mut current = start;

        let count_limit = self.count.unwrap_or(max_occurrences as u32).min(max_occurrences as u32);

        for _ in 0..count_limit {
            // Check until date if specified
            if let Some(until) = self.until {
                if current > until {
                    break;
                }
            }

            // Skip if weekday filter is set and this day doesn't match
            if let Some(ref weekdays) = self.by_weekday {
                if !weekdays.contains(&current.weekday()) {
                    // Advance without adding to results
                    match advance_by_frequency(current, self.frequency, self.interval) {
                        Some(next) => {
                            current = next;
                            continue;
                        }
                        None => break,
                    }
                }
            }

            occurrences.push(current);

            // Advance to next occurrence using shared helper
            match advance_by_frequency(current, self.frequency, self.interval) {
                Some(next) => current = next,
                None => break,
            };
        }

        Ok(occurrences)
    }

    /// Create a lazy iterator over occurrences of this recurrence pattern.
    ///
    /// Unlike [`generate_occurrences()`](Self::generate_occurrences), this method
    /// returns an iterator that computes each occurrence on demand, avoiding
    /// upfront memory allocation for large or infinite recurrence patterns.
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
/// Shared helper used by both the eager `generate_occurrences()` path and
/// the lazy `OccurrenceIterator`.
///
/// # Supported frequencies
///
/// - [`Frequency::Daily`] — adds `interval` days
/// - [`Frequency::Weekly`] — adds `interval` weeks
/// - [`Frequency::Monthly`] — adds `interval` months, clamping to the last
///   valid day if the target month is shorter (e.g. Jan 31 → Feb 28)
/// - [`Frequency::Yearly`] — adds `interval` years, clamping for leap-day
///   dates (e.g. Feb 29 → Feb 28 in non-leap years)
///
/// Other variants (`Secondly`, `Minutely`, `Hourly`) are **not supported**
/// and will return `None`, causing the iterator to terminate after the
/// first occurrence.
fn advance_by_frequency(
    current: DateTime<Tz>,
    frequency: Frequency,
    interval: u16,
) -> Option<DateTime<Tz>> {
    if interval == 0 {
        return None;
    }
    match frequency {
        Frequency::Daily => Some(current + chrono::Duration::days(interval as i64)),
        Frequency::Weekly => Some(current + chrono::Duration::weeks(interval as i64)),
        Frequency::Monthly => {
            let months_to_add = interval as i32;
            let mut new_month = current.month() as i32 + months_to_add;
            let mut new_year = current.year();
            while new_month > 12 {
                new_month -= 12;
                new_year += 1;
            }
            let date = clamp_day_to_month(new_year, new_month as u32, current.day())?;
            let naive = chrono::NaiveDateTime::new(date, current.time());
            current.timezone().from_local_datetime(&naive).earliest()
        }
        Frequency::Yearly => {
            let new_year = current.year() + interval as i32;
            let date = clamp_day_to_month(new_year, current.month(), current.day())?;
            let naive = chrono::NaiveDateTime::new(date, current.time());
            current.timezone().from_local_datetime(&naive).earliest()
        }
        _ => None,
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
    count: u32,
    exhausted: bool,
}

impl OccurrenceIterator {
    /// Create a new occurrence iterator
    fn new(recurrence: Recurrence, start: DateTime<Tz>) -> Self {
        Self {
            recurrence,
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
        advance_by_frequency(self.current, self.recurrence.frequency, self.recurrence.interval)
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

            // Compute next occurrence for future calls
            match self.compute_next() {
                Some(next) => {
                    self.current = next;
                    self.count += 1;
                }
                None => {
                    self.exhausted = true;
                    self.count += 1;
                }
            }

            // Skip if weekday filter is set and this day doesn't match
            if let Some(ref weekdays) = self.recurrence.by_weekday {
                if !weekdays.contains(&result.weekday()) {
                    continue;
                }
            }

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

        let eager: Vec<_> = recurrence.generate_occurrences(start, 10).unwrap();
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
        let eager = recurrence.generate_occurrences(start, 10).unwrap();
        assert_eq!(eager.len(), 1);
    }

    #[test]
    fn test_weekdays_filter_lazy() {
        use rrule::Weekday;
        // Weekly recurrence on Mon/Wed/Fri with daily interval
        // Start on a Monday (2025-01-06)
        let recurrence = Recurrence::daily()
            .weekdays(vec![Weekday::Mon, Weekday::Wed, Weekday::Fri])
            .count(14); // 14 daily slots, should yield ~6 matching days

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
        // 14 daily slots from Mon Jan 6: Mon6,Wed8,Fri10,Mon13,Wed15,Fri17 = 6
        assert_eq!(occurrences.len(), 6);
    }

    #[test]
    fn test_weekdays_filter_eager() {
        use rrule::Weekday;
        let recurrence = Recurrence::daily()
            .weekdays(vec![Weekday::Mon, Weekday::Wed, Weekday::Fri])
            .count(14);

        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-06 09:00:00", tz).unwrap();

        let eager = recurrence.generate_occurrences(start, 14).unwrap();
        let lazy: Vec<_> = recurrence.occurrences(start).collect();

        assert_eq!(eager.len(), lazy.len());
        for (e, l) in eager.iter().zip(lazy.iter()) {
            assert_eq!(e, l);
        }
    }
}
