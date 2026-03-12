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
    /// An interval of 0 is normalized to 1 (the RFC 5545 default).
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
        self.interval = if interval == 0 { 1 } else { interval };
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

    /// Set specific weekdays for the recurrence
    ///
    /// Behavior depends on the frequency:
    ///
    /// - **Weekly**: intra-week expansion — emits every listed weekday
    ///   within each week, then jumps by `interval` weeks. O(1) per emit.
    /// - **Daily**: direct weekday jumping — steps by `interval` days
    ///   until a matching weekday is found (at most 7 steps). O(1) per emit.
    /// - **Monthly**: RFC 5545 BYDAY period expansion — every matching
    ///   weekday within each recurrence month is produced. `interval`
    ///   controls which months are visited.
    /// - **Yearly**: RFC 5545 BYDAY period expansion — every matching
    ///   weekday within each recurrence year is produced.
    /// - **Sub-daily** (Hourly/Minutely/Secondly): cadence-preserving
    ///   weekday filter with O(1) day-skipping — when a step lands on a
    ///   non-matching day, the iterator jumps directly to the next
    ///   matching day instead of iterating through each sub-daily step.
    ///
    /// An empty list is normalized to no filter (ignored).
    ///
    /// # Examples
    ///
    /// ```
    /// use eventix::Recurrence;
    /// use rrule::Weekday;
    ///
    /// // Weekly: only on weekdays
    /// let weekdays = Recurrence::weekly()
    ///     .weekdays(vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri])
    ///     .count(20);
    ///
    /// // Monthly: every Tuesday and Thursday of each month
    /// let monthly_byday = Recurrence::monthly()
    ///     .weekdays(vec![Weekday::Tue, Weekday::Thu])
    ///     .count(20);
    /// ```
    pub fn weekdays(mut self, weekdays: Vec<rrule::Weekday>) -> Self {
        self.by_weekday = if weekdays.is_empty() {
            None
        } else {
            Some(weekdays)
        };
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

    /// Get the weekday filter of this recurrence
    pub fn get_weekdays(&self) -> Option<&[rrule::Weekday]> {
        self.by_weekday.as_deref()
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
            if !weekdays.is_empty() {
                let days: Vec<String> =
                    weekdays.iter().map(|wd| format!("{:?}", wd).to_uppercase()).collect();
                rrule_str.push_str(&format!(";BYDAY={}", days.join(",")));
            }
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

/// For Daily frequency with specific weekdays: advance to the next matching
/// weekday after `current`. Jumps directly to the target day, using
/// `intended_time` to reconstruct the wall-clock time on that day.
///
/// For `interval == 1` this is equivalent to stepping 1 day at a time but
/// short-circuits the scan. For `interval > 1` it steps by `interval`
/// days until a matching weekday is found (at most 7 steps because
/// weekdays repeat every 7 days, and any interval coprime to 7 covers
/// all weekdays within 7 steps).
fn advance_daily_weekday(
    current: DateTime<Tz>,
    interval: u16,
    weekdays: &[chrono::Weekday],
    intended_time: chrono::NaiveTime,
) -> Option<DateTime<Tz>> {
    if interval == 0 {
        return None;
    }
    let tz = current.timezone();
    let mut date = current.date_naive();
    // At most 7 interval-steps (all weekdays covered within one full cycle)
    for _ in 0..7 {
        date = date + chrono::Days::new(interval as u64);
        if weekdays.contains(&date.weekday()) {
            let naive = chrono::NaiveDateTime::new(date, intended_time);
            return resolve_local(tz, naive);
        }
    }
    None
}

/// For sub-daily frequencies on a non-matching weekday, compute the first
/// interval-aligned occurrence on the next matching weekday in O(1).
///
/// Without this, `secondly().interval(1).weekdays([Mon])` would iterate
/// 518,400 times to skip from Monday to the following Monday.
fn skip_subdaily_to_matching_day(
    current: DateTime<Tz>,
    frequency: Frequency,
    interval: u16,
    weekdays: &[chrono::Weekday],
) -> Option<DateTime<Tz>> {
    // If current already falls on a matching weekday, return it as-is.
    // This happens when compute_next() crosses midnight into a valid day.
    if weekdays.contains(&current.weekday()) {
        return Some(current);
    }

    let tz = current.timezone();
    let mut target_date = current.date_naive();

    // Find the next matching weekday (1-6 days ahead)
    let mut found = false;
    for _ in 0..7 {
        target_date = target_date.succ_opt()?;
        if weekdays.contains(&target_date.weekday()) {
            found = true;
            break;
        }
    }
    if !found {
        return None;
    }

    // Midnight of target day in the event's timezone
    let midnight_time = chrono::NaiveTime::from_hms_opt(0, 0, 0)?;
    let midnight = chrono::NaiveDateTime::new(target_date, midnight_time);
    let target_dt = resolve_local(tz, midnight)?;

    // Compute the number of interval-steps needed to reach or pass midnight.
    // Sub-daily uses UTC duration arithmetic, so signed_duration_since is exact.
    let gap_secs = target_dt.signed_duration_since(current).num_seconds();
    if gap_secs <= 0 {
        return Some(target_dt);
    }

    let interval_secs = match frequency {
        Frequency::Hourly => interval as i64 * 3600,
        Frequency::Minutely => interval as i64 * 60,
        Frequency::Secondly => interval as i64,
        _ => return None,
    };

    // Ceil division: first step at or past midnight
    let steps = (gap_secs + interval_secs - 1) / interval_secs;
    Some(current + chrono::Duration::seconds(steps * interval_secs))
}

/// Collect all dates in `(year, month)` that fall on one of the given
/// weekdays, resolved to timezone `tz` at wall-clock `time`.
/// Returns dates in calendar order.
fn expand_weekdays_in_month(
    year: i32,
    month: u32,
    weekdays: &[chrono::Weekday],
    tz: Tz,
    time: chrono::NaiveTime,
) -> Vec<DateTime<Tz>> {
    let mut results = Vec::with_capacity(5 * weekdays.len());
    let Some(first) = chrono::NaiveDate::from_ymd_opt(year, month, 1) else {
        return results;
    };
    // Last day of month: first day of next month - 1
    let last = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .and_then(|d| d.checked_sub_days(chrono::Days::new(1)))
    .unwrap_or(first);

    let mut date = first;
    loop {
        if weekdays.contains(&date.weekday()) {
            let naive = chrono::NaiveDateTime::new(date, time);
            if let Some(dt) = resolve_local(tz, naive) {
                results.push(dt);
            }
        }
        if date >= last {
            break;
        }
        date = match date.succ_opt() {
            Some(d) => d,
            None => break,
        };
    }
    results
}

/// Collect all dates in `year` that fall on one of the given weekdays,
/// resolved to timezone `tz` at wall-clock `time`.
/// Returns dates in calendar order.
fn expand_weekdays_in_year(
    year: i32,
    weekdays: &[chrono::Weekday],
    tz: Tz,
    time: chrono::NaiveTime,
) -> Vec<DateTime<Tz>> {
    let mut results = Vec::with_capacity(53 * weekdays.len());
    for month in 1..=12u32 {
        results.extend(expand_weekdays_in_month(year, month, weekdays, tz, time));
    }
    results
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
    /// Buffer for Monthly/Yearly BYDAY expansion (dates within current period)
    pending_byday: std::collections::VecDeque<DateTime<Tz>>,
    /// For BYDAY expansion: year of the next period to expand
    byday_next_year: i32,
    /// For BYDAY expansion: month of the next period to expand (Monthly only)
    byday_next_month: u32,
    /// Whether the first BYDAY period has been expanded
    byday_first: bool,
}

impl OccurrenceIterator {
    /// Create a new occurrence iterator
    fn new(recurrence: Recurrence, start: DateTime<Tz>) -> Self {
        Self {
            byday_next_year: start.year(),
            byday_next_month: start.month(),
            byday_first: true,
            pending_byday: std::collections::VecDeque::new(),
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

    /// Whether this iterator uses BYDAY period expansion (Monthly/Yearly + weekdays)
    fn uses_byday_expansion(&self) -> bool {
        matches!(self.recurrence.frequency, Frequency::Monthly | Frequency::Yearly)
            && self.recurrence.by_weekday.is_some()
    }

    /// Emit the next occurrence from BYDAY-expanded buffer,
    /// expanding new periods as needed.
    fn next_byday_expanded(&mut self) -> Option<DateTime<Tz>> {
        loop {
            // Try to emit from buffer
            if let Some(dt) = self.pending_byday.pop_front() {
                if let Some(max) = self.recurrence.count {
                    if self.count >= max {
                        return None;
                    }
                }
                if let Some(until) = self.recurrence.until {
                    if dt > until {
                        return None;
                    }
                }
                self.count += 1;
                return Some(dt);
            }

            // Buffer empty — expand next period
            if self.exhausted {
                return None;
            }
            self.expand_next_byday_period();
        }
    }

    /// Expand the next Monthly/Yearly period into `pending_byday`.
    fn expand_next_byday_period(&mut self) {
        let weekdays = match &self.recurrence.by_weekday {
            Some(wd) => wd.clone(),
            None => {
                self.exhausted = true;
                return;
            }
        };

        let tz = self.current.timezone();
        let is_first = self.byday_first;

        let dates = match self.recurrence.frequency {
            Frequency::Monthly => expand_weekdays_in_month(
                self.byday_next_year,
                self.byday_next_month,
                &weekdays,
                tz,
                self.intended_time,
            ),
            Frequency::Yearly => {
                expand_weekdays_in_year(self.byday_next_year, &weekdays, tz, self.intended_time)
            }
            _ => {
                self.exhausted = true;
                return;
            }
        };

        for dt in dates {
            // First period: only include dates >= start
            if is_first && dt < self.current {
                continue;
            }
            self.pending_byday.push_back(dt);
        }

        self.byday_first = false;

        // Advance to next period
        match self.recurrence.frequency {
            Frequency::Monthly => {
                let total = (self.byday_next_year as i64) * 12
                    + (self.byday_next_month as i64 - 1)
                    + self.recurrence.interval as i64;
                self.byday_next_year = (total / 12) as i32;
                self.byday_next_month = (total % 12 + 1) as u32;
            }
            Frequency::Yearly => {
                self.byday_next_year += self.recurrence.interval as i32;
            }
            _ => {}
        }

        // Safety: prevent runaway expansion beyond chrono's NaiveDate range
        if self.byday_next_year > 9999 {
            self.exhausted = true;
        }

        // Don't exhaust here: when the first period has no candidates
        // (start is past the last matching weekday), the next period
        // should be tried. The loop in next_byday_expanded() will call
        // expand_next_byday_period() again, and for valid weekday lists
        // every month/year has matching days. Count/until checks in
        // next_byday_expanded() guarantee termination for bounded
        // recurrences; for unbounded ones the caller must use .take()
        // or an until date.
    }
}

impl Iterator for OccurrenceIterator {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        // Fast path: no weekday filter active
        // Avoids per-iteration frequency checks that cause regression on
        // the common no-weekday path (daily, minutely, hourly, etc.)
        if self.recurrence.by_weekday.is_none() {
            if self.is_exhausted() {
                return None;
            }
            let result = self.current;
            match self.compute_next() {
                Some(next) => self.current = next,
                None => self.exhausted = true,
            }
            self.count += 1;
            return Some(result);
        }

        // Monthly/Yearly BYDAY: use period expansion instead of post-filter
        if self.uses_byday_expansion() {
            return self.next_byday_expanded();
        }

        // Weekday-filtered path — dispatch by frequency
        let weekdays = self.recurrence.by_weekday.as_ref()?;
        loop {
            if self.is_exhausted() {
                return None;
            }

            let result = self.current;

            match self.recurrence.frequency {
                // Weekly: intra-week expansion (O(1) per emit)
                Frequency::Weekly => {
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
                }
                // Daily: direct weekday jumping (O(1) per emit)
                Frequency::Daily => {
                    match advance_daily_weekday(
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
                }
                // Sub-daily: advance then O(1) skip over non-matching days
                Frequency::Hourly | Frequency::Minutely | Frequency::Secondly => {
                    match self.compute_next() {
                        Some(next) => self.current = next,
                        None => self.exhausted = true,
                    }
                    if !weekdays.contains(&result.weekday()) {
                        match skip_subdaily_to_matching_day(
                            self.current,
                            self.recurrence.frequency,
                            self.recurrence.interval,
                            weekdays,
                        ) {
                            Some(next) => self.current = next,
                            None => self.exhausted = true,
                        }
                    } else {
                        self.count += 1;
                        return Some(result);
                    }
                }
                // Monthly/Yearly without weekdays handled by BYDAY path above
                _ => {
                    match self.compute_next() {
                        Some(next) => self.current = next,
                        None => self.exhausted = true,
                    }
                    self.count += 1;
                    return Some(result);
                }
            }
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
    fn test_zero_interval_clamped_to_one() {
        // interval(0) is normalized to 1 (RFC 5545 default)
        let recurrence = Recurrence::daily().interval(0).count(10);
        assert_eq!(recurrence.get_interval(), 1);

        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        // Should behave identically to interval(1)
        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 10);
        assert_eq!(occurrences[0], start);
    }

    #[test]
    fn test_zero_interval_weekly_weekdays_clamped_to_one() {
        // interval(0) on weekly + weekdays is normalized to interval(1)
        let recurrence = Recurrence::weekly()
            .interval(0)
            .weekdays(vec![chrono::Weekday::Mon, chrono::Weekday::Wed])
            .count(5);
        assert_eq!(recurrence.get_interval(), 1);

        let tz = parse_timezone("UTC").unwrap();
        // Start on a Monday
        let start = crate::timezone::parse_datetime_with_tz("2025-01-06 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        // Should produce 5 occurrences (Mon and Wed each week)
        assert_eq!(occurrences.len(), 5);
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

    #[test]
    fn test_interval_zero_rrule_string_consistent() {
        // interval(0) is clamped to 1, so RRULE should omit INTERVAL (default=1)
        let recurrence = Recurrence::daily().interval(0).count(5);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let rrule_str = recurrence.to_rrule_string(start).unwrap();
        assert!(
            !rrule_str.contains("INTERVAL=0"),
            "interval(0) should not appear in RRULE, got: {}",
            rrule_str
        );
        // Behavior matches interval(1)
        let r1 = Recurrence::daily().interval(1).count(5);
        let rrule_str1 = r1.to_rrule_string(start).unwrap();
        assert_eq!(rrule_str, rrule_str1);
    }

    #[test]
    fn test_empty_weekdays_normalized_to_none() {
        // weekdays(vec![]) should be normalized to None (no filter)
        let recurrence = Recurrence::daily().weekdays(vec![]).count(5);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        // Should produce 5 occurrences, not hang
        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 5);

        // RRULE should not contain BYDAY
        let rrule_str = recurrence.to_rrule_string(start).unwrap();
        assert!(
            !rrule_str.contains("BYDAY"),
            "empty weekdays should not emit BYDAY, got: {}",
            rrule_str
        );
    }

    #[test]
    fn test_monthly_byday_expansion() {
        use chrono::Weekday;
        // Monthly + BYDAY=TU: every Tuesday of each month
        let recurrence = Recurrence::monthly().weekdays(vec![Weekday::Tue]).count(10);
        let tz = parse_timezone("UTC").unwrap();
        // Start on Jan 1 2025 (Wednesday)
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 10);

        // All must be Tuesdays
        for occ in &occurrences {
            assert_eq!(occ.weekday(), Weekday::Tue, "expected Tuesday, got {:?}", occ);
        }

        // First Tuesday of Jan 2025 >= Jan 1 is Jan 7
        assert_eq!(occurrences[0].day(), 7);
        // Jan has Tuesdays: 7, 14, 21, 28 (4 total)
        assert_eq!(occurrences[1].day(), 14);
        assert_eq!(occurrences[2].day(), 21);
        assert_eq!(occurrences[3].day(), 28);
        // 5th occurrence: first Tuesday of Feb = Feb 4
        assert_eq!(occurrences[4].month(), 2);
        assert_eq!(occurrences[4].day(), 4);
    }

    #[test]
    fn test_monthly_byday_multiple_weekdays() {
        use chrono::Weekday;
        // Monthly + BYDAY=TU,TH: every Tuesday and Thursday of each month
        let recurrence = Recurrence::monthly().weekdays(vec![Weekday::Tue, Weekday::Thu]).count(12);
        let tz = parse_timezone("UTC").unwrap();
        // Start on Jan 1 2025 (Wednesday)
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 12);

        // All must be Tuesday or Thursday
        for occ in &occurrences {
            let wd = occ.weekday();
            assert!(
                wd == Weekday::Tue || wd == Weekday::Thu,
                "expected Tue or Thu, got {:?} on {}",
                wd,
                occ
            );
        }

        // Jan 2025: Thu 2, Tue 7, Thu 9, Tue 14, Thu 16, Tue 21, Thu 23, Tue 28, Thu 30
        // First >= Jan 1: Thu Jan 2
        assert_eq!(occurrences[0].day(), 2);
        assert_eq!(occurrences[0].weekday(), Weekday::Thu);
        assert_eq!(occurrences[1].day(), 7);
        assert_eq!(occurrences[1].weekday(), Weekday::Tue);
    }

    #[test]
    fn test_monthly_byday_with_interval() {
        use chrono::Weekday;
        // Every 2 months, Mondays only
        let recurrence = Recurrence::monthly().interval(2).weekdays(vec![Weekday::Mon]).count(10);
        let tz = parse_timezone("UTC").unwrap();
        // Start Jan 1 2025 (Wed)
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 10);

        // All Mondays
        for occ in &occurrences {
            assert_eq!(occ.weekday(), Weekday::Mon);
        }

        // Jan Mondays: 6, 13, 20, 27 (4)
        // Skip Feb, next period is Mar
        // Mar Mondays: 3, 10, 17, 24, 31 (5)
        // So first 4 in Jan, then 5 in Mar (total 9), then May for #10
        assert_eq!(occurrences[0].month(), 1);
        assert_eq!(occurrences[0].day(), 6);
        assert_eq!(occurrences[3].month(), 1);
        assert_eq!(occurrences[3].day(), 27);
        // 5th occurrence: first Monday of March
        assert_eq!(occurrences[4].month(), 3);
        assert_eq!(occurrences[4].day(), 3);
    }

    #[test]
    fn test_monthly_byday_start_on_matching_weekday() {
        use chrono::Weekday;
        // Start on a Tuesday — it should be included
        let recurrence = Recurrence::monthly().weekdays(vec![Weekday::Tue]).count(5);
        let tz = parse_timezone("UTC").unwrap();
        // Jan 7 2025 is a Tuesday
        let start = crate::timezone::parse_datetime_with_tz("2025-01-07 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 5);
        // Start date should be included
        assert_eq!(occurrences[0], start);
        // Next Tuesdays: 14, 21, 28, then Feb 4
        assert_eq!(occurrences[1].day(), 14);
        assert_eq!(occurrences[4].month(), 2);
        assert_eq!(occurrences[4].day(), 4);
    }

    #[test]
    fn test_monthly_byday_with_until() {
        use chrono::Weekday;
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();
        let until = crate::timezone::parse_datetime_with_tz("2025-01-20 23:59:59", tz).unwrap();

        let recurrence = Recurrence::monthly().weekdays(vec![Weekday::Tue]).until(until);

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        // Jan Tuesdays <= Jan 20: 7, 14
        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].day(), 7);
        assert_eq!(occurrences[1].day(), 14);
    }

    #[test]
    fn test_yearly_byday_expansion() {
        use chrono::Weekday;
        // Yearly + BYDAY=MO: every Monday of each year
        let recurrence = Recurrence::yearly().weekdays(vec![Weekday::Mon]).count(5);
        let tz = parse_timezone("UTC").unwrap();
        // Start Jan 1 2025 (Wednesday)
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 5);

        for occ in &occurrences {
            assert_eq!(occ.weekday(), Weekday::Mon);
        }

        // First Monday >= Jan 1 2025 is Jan 6
        assert_eq!(occurrences[0].day(), 6);
        assert_eq!(occurrences[0].month(), 1);
        assert_eq!(occurrences[1].day(), 13);
    }

    #[test]
    fn test_yearly_byday_with_interval() {
        use chrono::Weekday;
        // Every 2 years, Fridays only, count=3
        let recurrence = Recurrence::yearly().interval(2).weekdays(vec![Weekday::Fri]).count(3);
        let tz = parse_timezone("UTC").unwrap();
        // Start Jan 1 2025 (Wednesday)
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 3);

        for occ in &occurrences {
            assert_eq!(occ.weekday(), Weekday::Fri);
        }

        // First Friday >= Jan 1 2025 is Jan 3
        assert_eq!(occurrences[0].year(), 2025);
        assert_eq!(occurrences[0].month(), 1);
        assert_eq!(occurrences[0].day(), 3);
    }

    #[test]
    fn test_monthly_byday_eager_matches_lazy() {
        use chrono::Weekday;
        let recurrence = Recurrence::monthly().weekdays(vec![Weekday::Wed, Weekday::Fri]).count(15);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let lazy: Vec<_> = recurrence.clone().occurrences(start).collect();
        let eager = recurrence.generate_occurrences(start).unwrap();

        assert_eq!(lazy.len(), 15);
        assert_eq!(lazy, eager);
    }

    #[test]
    fn test_monthly_byday_dst_spring_forward() {
        use chrono::Weekday;
        // Monthly Sundays across US spring-forward (Mar 9 2025 at 2:00 AM)
        let recurrence = Recurrence::monthly().weekdays(vec![Weekday::Sun]).count(12);
        let tz = parse_timezone("America/New_York").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-03-01 02:30:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 12);

        for occ in &occurrences {
            assert_eq!(occ.weekday(), Weekday::Sun);
        }

        // Mar 9 2025 is a Sunday — 2:30 AM doesn't exist (DST gap).
        // resolve_local should handle it gracefully.
        let mar_9 = occurrences.iter().find(|o| o.month() == 3 && o.day() == 9);
        assert!(mar_9.is_some(), "Mar 9 (spring forward) should be present");
    }

    #[test]
    fn test_daily_weekday_direct_jump() {
        use chrono::Weekday;
        // Daily interval=1, weekdays=[Mon, Wed, Fri], count=9
        let recurrence = Recurrence::daily()
            .weekdays(vec![Weekday::Mon, Weekday::Wed, Weekday::Fri])
            .count(9);
        let tz = parse_timezone("UTC").unwrap();
        // Start Saturday Jan 4 2025 — not a matching day
        let start = crate::timezone::parse_datetime_with_tz("2025-01-04 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 9);

        for occ in &occurrences {
            let wd = occ.weekday();
            assert!(
                wd == Weekday::Mon || wd == Weekday::Wed || wd == Weekday::Fri,
                "expected Mon/Wed/Fri, got {:?} on {}",
                wd,
                occ
            );
        }

        // First match after Sat Jan 4: Mon Jan 6
        assert_eq!(occurrences[0].day(), 6);
        assert_eq!(occurrences[0].weekday(), Weekday::Mon);
    }

    #[test]
    fn test_daily_interval2_weekday_jump() {
        use chrono::Weekday;
        // Daily interval=2, weekdays=[Tue], count=5
        let recurrence = Recurrence::daily().interval(2).weekdays(vec![Weekday::Tue]).count(5);
        let tz = parse_timezone("UTC").unwrap();
        // Start Wed Jan 1 2025
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 5);

        for occ in &occurrences {
            assert_eq!(occ.weekday(), Weekday::Tue);
        }
    }

    #[test]
    fn test_daily_weekday_eager_matches_lazy() {
        use chrono::Weekday;
        let recurrence = Recurrence::daily()
            .weekdays(vec![Weekday::Mon, Weekday::Wed, Weekday::Fri])
            .count(15);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-01 09:00:00", tz).unwrap();

        let lazy: Vec<_> = recurrence.clone().occurrences(start).collect();
        let eager = recurrence.generate_occurrences(start).unwrap();

        assert_eq!(lazy.len(), 15);
        assert_eq!(lazy, eager);
    }

    #[test]
    fn test_subdaily_weekday_skip() {
        use chrono::Weekday;
        // Hourly interval=1, weekdays=[Mon], count=48
        // Should emit 24 hours on first Mon, skip Tue-Sun in O(1), emit 24 on next Mon
        let recurrence = Recurrence::hourly().interval(1).weekdays(vec![Weekday::Mon]).count(48);
        let tz = parse_timezone("UTC").unwrap();
        // Start Mon Jan 6 2025 00:00
        let start = crate::timezone::parse_datetime_with_tz("2025-01-06 00:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 48);

        // All must be Monday
        for occ in &occurrences {
            assert_eq!(occ.weekday(), Weekday::Mon, "expected Monday, got {:?}", occ);
        }

        // First 24 on Jan 6, next 24 on Jan 13
        assert_eq!(occurrences[0].day(), 6);
        assert_eq!(occurrences[23].day(), 6);
        assert_eq!(occurrences[23].hour(), 23);
        assert_eq!(occurrences[24].day(), 13);
        assert_eq!(occurrences[24].hour(), 0);
    }

    #[test]
    fn test_subdaily_weekday_minutely_skip() {
        use chrono::Weekday;
        // Minutely interval=30, weekdays=[Tue, Thu], count=96
        // Each matching day has 48 half-hour slots (0:00..23:30)
        let recurrence = Recurrence::minutely()
            .interval(30)
            .weekdays(vec![Weekday::Tue, Weekday::Thu])
            .count(96);
        let tz = parse_timezone("UTC").unwrap();
        // Start Tue Jan 7 2025 00:00
        let start = crate::timezone::parse_datetime_with_tz("2025-01-07 00:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 96);

        for occ in &occurrences {
            let wd = occ.weekday();
            assert!(wd == Weekday::Tue || wd == Weekday::Thu, "expected Tue/Thu, got {:?}", wd);
        }

        // First 48 on Tue Jan 7, next 48 on Thu Jan 9
        assert_eq!(occurrences[0].day(), 7);
        assert_eq!(occurrences[47].day(), 7);
        assert_eq!(occurrences[48].day(), 9);
    }

    #[test]
    fn test_subdaily_weekday_secondly_perf() {
        use chrono::Weekday;
        // Secondly interval=1, weekdays=[Wed], count=10
        // Without O(1) skip this would iterate 86400*6=518400 to cross 6 days.
        // With the skip, it computes the jump directly.
        let recurrence = Recurrence::secondly().interval(1).weekdays(vec![Weekday::Wed]).count(10);
        let tz = parse_timezone("UTC").unwrap();
        // Start Thu Jan 2 2025 00:00:00 — not a Wednesday
        let start = crate::timezone::parse_datetime_with_tz("2025-01-02 00:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();

        assert_eq!(occurrences.len(), 10);

        for occ in &occurrences {
            assert_eq!(occ.weekday(), Weekday::Wed);
        }

        // First Wednesday after Thu Jan 2 = Wed Jan 8
        assert_eq!(occurrences[0].day(), 8);
        assert_eq!(occurrences[0].month(), 1);
    }

    #[test]
    fn test_subdaily_weekday_start_on_matching_day() {
        use chrono::Weekday;
        // Start on a matching Wednesday — should include start
        let recurrence = Recurrence::hourly().interval(4).weekdays(vec![Weekday::Wed]).count(6);
        let tz = parse_timezone("UTC").unwrap();
        // Wed Jan 8 2025
        let start = crate::timezone::parse_datetime_with_tz("2025-01-08 08:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 6);

        // Start included: 08:00, 12:00, 16:00, 20:00 = 4 on this Wed
        // Then next Wed: 4 more starting from midnight-ish (first aligned slot)
        assert_eq!(occurrences[0], start);
        for occ in &occurrences {
            assert_eq!(occ.weekday(), Weekday::Wed);
        }
    }

    #[test]
    fn test_byday_monthly_start_past_last_weekday() {
        use chrono::Weekday;
        // Start on Jan 31 (Friday): all Mon/Wed in January are before this date.
        // The iterator must NOT exhaust — it should advance to February.
        let recurrence = Recurrence::monthly().weekdays(vec![Weekday::Mon]).count(2);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-01-31 10:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 2);
        // First Monday in Feb 2025 = Feb 3
        assert_eq!(occurrences[0].day(), 3);
        assert_eq!(occurrences[0].month(), 2);
        assert_eq!(occurrences[0].weekday(), Weekday::Mon);
        // Second occurrence: next Monday = Feb 10
        assert_eq!(occurrences[1].day(), 10);
        assert_eq!(occurrences[1].month(), 2);
    }

    #[test]
    fn test_byday_yearly_start_past_all_weekdays() {
        use chrono::Weekday;
        // Start on Dec 31 — all Mondays in 2025 are before this.
        // Must advance to 2026.
        let recurrence = Recurrence::yearly().weekdays(vec![Weekday::Mon]).count(1);
        let tz = parse_timezone("UTC").unwrap();
        let start = crate::timezone::parse_datetime_with_tz("2025-12-31 10:00:00", tz).unwrap();

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 1);
        // First Monday in 2026 = Jan 5
        assert_eq!(occurrences[0].year(), 2026);
        assert_eq!(occurrences[0].month(), 1);
        assert_eq!(occurrences[0].day(), 5);
    }

    #[test]
    fn test_subdaily_skip_does_not_overshoot_matching_day() {
        use chrono::Datelike;
        // Start Saturday 23:00, hourly, weekdays=[Sun, Mon].
        // Second occurrence crosses midnight into Sunday — must NOT skip Sunday.
        let tz = parse_timezone("UTC").unwrap();
        // 2025-06-07 is a Saturday
        let start = crate::timezone::parse_datetime_with_tz("2025-06-07 23:00:00", tz).unwrap();
        assert_eq!(start.weekday(), chrono::Weekday::Sat);

        let recurrence = Recurrence::hourly()
            .interval(1)
            .weekdays(vec![chrono::Weekday::Sun, chrono::Weekday::Mon])
            .count(3);

        let occurrences: Vec<_> = recurrence.occurrences(start).collect();
        assert_eq!(occurrences.len(), 3);
        // First occurrence: Sunday 00:00 (first valid slot after Sat 23:00)
        assert_eq!(occurrences[0].weekday(), chrono::Weekday::Sun);
        assert_eq!(occurrences[0].day(), 8);
        assert_eq!(occurrences[0].hour(), 0);
        // Second: Sunday 01:00
        assert_eq!(occurrences[1].hour(), 1);
        // Third: Sunday 02:00
        assert_eq!(occurrences[2].hour(), 2);
    }
}
