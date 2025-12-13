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

    /// Generate occurrences for this recurrence pattern
    ///
    /// Returns a vector of `DateTime<Tz>` representing each occurrence
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

            occurrences.push(current);

            // Calculate next occurrence based on frequency
            current = match self.frequency {
                Frequency::Daily => current + chrono::Duration::days(self.interval as i64),
                Frequency::Weekly => current + chrono::Duration::weeks(self.interval as i64),
                Frequency::Monthly => {
                    // Add months
                    let months_to_add = self.interval as i32;
                    let mut new_month = current.month() as i32 + months_to_add;
                    let mut new_year = current.year();

                    while new_month > 12 {
                        new_month -= 12;
                        new_year += 1;
                    }

                    let new_date = current
                        .date_naive()
                        .with_year(new_year)
                        .and_then(|d| d.with_month(new_month as u32));

                    match new_date {
                        Some(date) => {
                            let time = current.time();
                            let naive = chrono::NaiveDateTime::new(date, time);
                            current
                                .timezone()
                                .from_local_datetime(&naive)
                                .earliest()
                                .unwrap_or(current)
                        }
                        None => break,
                    }
                }
                Frequency::Yearly => {
                    let new_year = current.year() + self.interval as i32;
                    let new_date = current.date_naive().with_year(new_year);

                    match new_date {
                        Some(date) => {
                            let time = current.time();
                            let naive = chrono::NaiveDateTime::new(date, time);
                            current
                                .timezone()
                                .from_local_datetime(&naive)
                                .earliest()
                                .unwrap_or(current)
                        }
                        None => break,
                    }
                }
                _ => break, // Unsupported frequency
            };
        }

        Ok(occurrences)
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
}
