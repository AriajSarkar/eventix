//! Calendar day/week view iterators for UI-friendly rendering.
//!
//! This module lifts the crate's existing occurrence expansion into lazy
//! calendar-level traversal primitives. Each iterator item owns its event
//! metadata, making it easy to collect and move into UI component props.
//! Iteration is fallible: each item is yielded as a [`crate::Result`] so
//! callers can handle calendar expansion errors explicitly.

use crate::calendar::{Calendar, EventOccurrence};
use crate::error::{EventixError, Result};
use crate::event::EventStatus;
use crate::timezone::local_day_window;
use crate::{DateTime, Duration, Tz};
use chrono::{Datelike, Days, NaiveDate};
use std::cmp::Ordering;
use std::iter::FusedIterator;

/// An owned occurrence snapshot detached from the source calendar borrow.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedEventOccurrence {
    /// Index of the event in the calendar's event list.
    pub event_index: usize,
    /// Snapshot of the event title.
    pub title: String,
    /// Snapshot of the optional event description.
    pub description: Option<String>,
    /// Snapshot of the optional event location.
    pub location: Option<String>,
    /// Event booking status at iteration time.
    pub status: EventStatus,
    /// When this occurrence starts.
    pub occurrence_time: DateTime<Tz>,
    /// Duration of the occurrence.
    pub duration: Duration,
}

impl OwnedEventOccurrence {
    fn from_occurrence(occurrence: EventOccurrence<'_>) -> Self {
        Self {
            event_index: occurrence.event_index,
            title: occurrence.event.title.clone(),
            description: occurrence.event.description.clone(),
            location: occurrence.event.location.clone(),
            status: occurrence.event.status,
            occurrence_time: occurrence.occurrence_time,
            duration: occurrence.event.duration(),
        }
    }

    /// End time of this occurrence.
    pub fn end_time(&self) -> DateTime<Tz> {
        self.occurrence_time + self.duration
    }

    /// Title of this occurrence.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Description of this occurrence.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl PartialOrd for OwnedEventOccurrence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OwnedEventOccurrence {
    fn cmp(&self, other: &Self) -> Ordering {
        self.occurrence_time
            .cmp(&other.occurrence_time)
            .then_with(|| self.event_index.cmp(&other.event_index))
            .then_with(|| self.title.cmp(&other.title))
            .then_with(|| self.description.cmp(&other.description))
            .then_with(|| self.location.cmp(&other.location))
            .then_with(|| self.status.cmp(&other.status))
            .then_with(|| self.duration.cmp(&other.duration))
    }
}

/// A single calendar day with all active events whose time span intersects the
/// local day pre-bucketed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DayView {
    date: NaiveDate,
    timezone: Tz,
    start: DateTime<Tz>,
    end_exclusive: DateTime<Tz>,
    events: Vec<OwnedEventOccurrence>,
}

impl DayView {
    fn new(
        date: NaiveDate,
        timezone: Tz,
        start: DateTime<Tz>,
        end_exclusive: DateTime<Tz>,
        events: Vec<OwnedEventOccurrence>,
    ) -> Self {
        Self {
            date,
            timezone,
            start,
            end_exclusive,
            events,
        }
    }

    /// The calendar date for this day view.
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    /// The timezone used to compute this day.
    pub fn timezone(&self) -> Tz {
        self.timezone
    }

    /// All active events intersecting this day, sorted by occurrence start time.
    pub fn events(&self) -> &[OwnedEventOccurrence] {
        &self.events
    }

    /// Number of events intersecting this day.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Whether this day has no active events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Start of the day in the configured timezone.
    pub fn start(&self) -> DateTime<Tz> {
        self.start
    }

    /// Exclusive end of the day in the configured timezone.
    ///
    /// This is the next day's midnight and matches the half-open interval used
    /// by [`Calendar::events_between`](crate::Calendar::events_between).
    pub fn end(&self) -> DateTime<Tz> {
        self.end_exclusive
    }

    /// Exclusive end of the day in the configured timezone.
    pub fn end_exclusive(&self) -> DateTime<Tz> {
        self.end_exclusive
    }

    /// Inclusive end of the day for display-only scenarios.
    ///
    /// This subtracts one nanosecond from the exclusive boundary. Use
    /// [`DayView::end()`] or [`DayView::end_exclusive()`] for computations.
    pub fn end_inclusive(&self) -> DateTime<Tz> {
        if self.end_exclusive > self.start {
            self.end_exclusive - Duration::nanoseconds(1)
        } else {
            self.start
        }
    }
}

/// A calendar week containing seven day views.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WeekView {
    days: [DayView; 7],
}

impl WeekView {
    fn new(days: [DayView; 7]) -> Self {
        Self { days }
    }

    /// The seven day views in this week.
    pub fn days(&self) -> &[DayView; 7] {
        &self.days
    }

    /// Date of the first day in the week.
    pub fn start_date(&self) -> NaiveDate {
        self.days[0].date()
    }

    /// Date of the last day in the week.
    pub fn end_date(&self) -> NaiveDate {
        self.days[6].date()
    }

    /// Total number of events across all seven days.
    pub fn event_count(&self) -> usize {
        self.days.iter().map(DayView::event_count).sum()
    }

    /// All events across the week, flattened and sorted by occurrence time.
    pub fn all_events(&self) -> Vec<&OwnedEventOccurrence> {
        let mut events: Vec<_> = self.days.iter().flat_map(|day| day.events()).collect();
        events.sort();
        events
    }

    /// Whether every day in the week is empty.
    pub fn is_empty(&self) -> bool {
        self.days.iter().all(DayView::is_empty)
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

/// A lazy iterator over calendar days.
#[derive(Debug, Clone)]
pub struct DayIterator<'a> {
    calendar: &'a Calendar,
    current_date: Option<NaiveDate>,
    timezone: Tz,
    direction: Direction,
}

impl<'a> DayIterator<'a> {
    pub(crate) fn new(calendar: &'a Calendar, start: DateTime<Tz>) -> Self {
        Self::from_date(calendar, start.date_naive(), start.timezone(), Direction::Forward)
    }

    pub(crate) fn backward(calendar: &'a Calendar, start: DateTime<Tz>) -> Self {
        Self::from_date(calendar, start.date_naive(), start.timezone(), Direction::Backward)
    }

    fn from_date(
        calendar: &'a Calendar,
        date: NaiveDate,
        timezone: Tz,
        direction: Direction,
    ) -> Self {
        Self {
            calendar,
            current_date: Some(date),
            timezone,
            direction,
        }
    }

    /// Move the iterator to a new calendar date in the same timezone.
    ///
    /// This resets the cursor unconditionally. In a forward iterator,
    /// jumping to an earlier date will re-visit dates that may already
    /// have been yielded, and vice versa for backward iteration.
    pub fn skip_to(&mut self, date: NaiveDate) {
        self.current_date = Some(date);
    }
}

impl Iterator for DayIterator<'_> {
    type Item = Result<DayView>;

    fn next(&mut self) -> Option<Self::Item> {
        let date = self.current_date?;
        self.current_date = advance_date(date, self.direction);
        Some(build_day_view(self.calendar, date, self.timezone))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining =
            self.current_date.map(|date| remaining_days(date, self.direction)).unwrap_or(0);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for DayIterator<'_> {
    fn len(&self) -> usize {
        self.current_date.map(|date| remaining_days(date, self.direction)).unwrap_or(0)
    }
}

impl FusedIterator for DayIterator<'_> {}

/// A lazy iterator over calendar weeks.
#[derive(Debug, Clone)]
pub struct WeekIterator<'a> {
    calendar: &'a Calendar,
    current_week_start: Option<NaiveDate>,
    timezone: Tz,
    direction: Direction,
}

impl<'a> WeekIterator<'a> {
    pub(crate) fn new(calendar: &'a Calendar, start: DateTime<Tz>) -> Self {
        Self::from_date(calendar, start.date_naive(), start.timezone(), Direction::Forward)
    }

    pub(crate) fn backward(calendar: &'a Calendar, start: DateTime<Tz>) -> Self {
        Self::from_date(calendar, start.date_naive(), start.timezone(), Direction::Backward)
    }

    fn from_date(
        calendar: &'a Calendar,
        date: NaiveDate,
        timezone: Tz,
        direction: Direction,
    ) -> Self {
        Self {
            calendar,
            current_week_start: aligned_full_week_start(date),
            timezone,
            direction,
        }
    }

    /// Move the iterator to the week containing `date`.
    ///
    /// This resets the cursor unconditionally. In a forward iterator,
    /// jumping to an earlier date will re-visit week windows that may
    /// already have been yielded, and vice versa for backward iteration.
    /// If `date` falls so close to the supported upper bound that a full
    /// Monday-Sunday window cannot be formed, the iterator becomes empty.
    pub fn skip_to(&mut self, date: NaiveDate) {
        self.current_week_start = aligned_full_week_start(date);
    }
}

impl Iterator for WeekIterator<'_> {
    type Item = Result<WeekView>;

    fn next(&mut self) -> Option<Self::Item> {
        let week_start = self.current_week_start?;
        self.current_week_start = advance_week_start(week_start, self.direction);
        Some(build_week_view(self.calendar, week_start, self.timezone))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self
            .current_week_start
            .map(|week_start| remaining_full_weeks(week_start, self.direction))
            .unwrap_or(0);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for WeekIterator<'_> {
    fn len(&self) -> usize {
        self.current_week_start
            .map(|week_start| remaining_full_weeks(week_start, self.direction))
            .unwrap_or(0)
    }
}

impl FusedIterator for WeekIterator<'_> {}

fn advance_date(date: NaiveDate, direction: Direction) -> Option<NaiveDate> {
    match direction {
        Direction::Forward => date.succ_opt(),
        Direction::Backward => date.pred_opt(),
    }
}

fn build_day_view(calendar: &Calendar, date: NaiveDate, timezone: Tz) -> Result<DayView> {
    let (start, end_exclusive) = local_day_window(date, timezone)?;
    let events = calendar
        .events_between(start, end_exclusive)?
        .into_iter()
        .filter(|occurrence| occurrence.event.is_active())
        .map(OwnedEventOccurrence::from_occurrence)
        .collect();

    Ok(DayView::new(date, timezone, start, end_exclusive, events))
}

fn build_week_view(calendar: &Calendar, week_start: NaiveDate, timezone: Tz) -> Result<WeekView> {
    let mut day_iter = DayIterator::from_date(calendar, week_start, timezone, Direction::Forward);
    let mut days = Vec::with_capacity(7);

    for _ in 0..7 {
        let Some(day) = day_iter.next() else {
            return Err(EventixError::ValidationError(
                "Could not construct a full Monday-Sunday week window".to_string(),
            ));
        };
        days.push(day?);
    }

    let days = days.try_into().map_err(|_| {
        EventixError::ValidationError("Week views must contain exactly seven days".to_string())
    })?;

    Ok(WeekView::new(days))
}

fn align_to_monday(date: NaiveDate) -> Option<NaiveDate> {
    let days_since_monday = date.weekday().num_days_from_monday() as u64;
    date.checked_sub_days(Days::new(days_since_monday))
}

fn aligned_full_week_start(date: NaiveDate) -> Option<NaiveDate> {
    let monday = align_to_monday(date)?;
    if monday.checked_add_days(Days::new(6)).is_some() {
        Some(monday)
    } else {
        None
    }
}

fn advance_week_start(week_start: NaiveDate, direction: Direction) -> Option<NaiveDate> {
    let next = match direction {
        Direction::Forward => week_start.checked_add_days(Days::new(7)),
        Direction::Backward => week_start.checked_sub_days(Days::new(7)),
    }?;

    if next.checked_add_days(Days::new(6)).is_some() {
        Some(next)
    } else {
        None
    }
}

fn remaining_days(date: NaiveDate, direction: Direction) -> usize {
    let remaining = match direction {
        Direction::Forward => NaiveDate::MAX.signed_duration_since(date).num_days(),
        Direction::Backward => date.signed_duration_since(NaiveDate::MIN).num_days(),
    };

    remaining.try_into().unwrap_or(usize::MAX).saturating_add(1)
}

fn remaining_full_weeks(week_start: NaiveDate, direction: Direction) -> usize {
    let remaining = match direction {
        Direction::Forward => last_full_week_start().signed_duration_since(week_start).num_days(),
        Direction::Backward => week_start.signed_duration_since(first_full_week_start()).num_days(),
    };

    if remaining < 0 {
        return 0;
    }

    remaining.try_into().unwrap_or(usize::MAX).saturating_div(7).saturating_add(1)
}

fn first_full_week_start() -> NaiveDate {
    let days_until_monday = (7 - NaiveDate::MIN.weekday().num_days_from_monday()) % 7;
    NaiveDate::MIN
        .checked_add_days(Days::new(days_until_monday as u64))
        .unwrap_or(NaiveDate::MIN)
}

fn last_full_week_start() -> NaiveDate {
    let latest_start_candidate =
        NaiveDate::MAX.checked_sub_days(Days::new(6)).unwrap_or(NaiveDate::MAX);
    align_to_monday(latest_start_candidate).unwrap_or(latest_start_candidate)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::{timezone, Calendar, Duration, Event, EventStatus, Recurrence};
    use chrono::Timelike;

    fn collect_ok<T>(iter: impl Iterator<Item = Result<T>>) -> Vec<T> {
        iter.collect::<Result<Vec<_>>>().unwrap()
    }

    fn next_ok<T>(mut iter: impl Iterator<Item = Result<T>>) -> T {
        iter.next().unwrap().unwrap()
    }

    fn sample_calendar() -> Calendar {
        let mut calendar = Calendar::new("Views");

        calendar.add_event(
            Event::builder()
                .title("Planning")
                .description("Weekly planning")
                .location("Room A")
                .start("2025-11-03 09:00:00", "America/New_York")
                .duration_hours(1)
                .build()
                .unwrap(),
        );

        calendar.add_event(
            Event::builder()
                .title("Standup")
                .start("2025-11-04 10:00:00", "America/New_York")
                .duration_minutes(15)
                .recurrence(Recurrence::daily().count(5))
                .build()
                .unwrap(),
        );

        calendar.add_event(
            Event::builder()
                .title("Cancelled")
                .start("2025-11-05 11:00:00", "America/New_York")
                .duration_minutes(30)
                .status(EventStatus::Cancelled)
                .build()
                .unwrap(),
        );

        calendar
    }

    #[test]
    fn test_day_iterator_basic() {
        let calendar = Calendar::new("Basic");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-01 12:00:00", tz).unwrap();

        let days = collect_ok(calendar.days(start).take(3));

        assert_eq!(
            days.iter().map(DayView::date).collect::<Vec<_>>(),
            vec![
                chrono::NaiveDate::from_ymd_opt(2025, 11, 1).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 2).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 3).unwrap(),
            ]
        );
    }

    #[test]
    fn test_day_iterator_with_events() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-03 00:00:00", tz).unwrap();

        let days = collect_ok(calendar.days(start).take(3));

        assert_eq!(days[0].event_count(), 1);
        assert_eq!(days[0].events()[0].title(), "Planning");
        assert_eq!(days[1].event_count(), 1);
        assert_eq!(days[1].events()[0].title(), "Standup");
    }

    #[test]
    fn test_day_iterator_backward() {
        let calendar = Calendar::new("Backward");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-03 12:00:00", tz).unwrap();

        let days = collect_ok(calendar.days_back(start).take(3));

        assert_eq!(
            days.iter().map(DayView::date).collect::<Vec<_>>(),
            vec![
                chrono::NaiveDate::from_ymd_opt(2025, 11, 3).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 2).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 1).unwrap(),
            ]
        );
    }

    #[test]
    fn test_day_iterator_recurring_events() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-04 00:00:00", tz).unwrap();

        let days = collect_ok(calendar.days(start).take(5));

        assert!(days
            .iter()
            .all(|day| day.events().iter().all(|event| event.title() == "Standup")));
        assert_eq!(days.iter().map(DayView::event_count).sum::<usize>(), 5);
    }

    #[test]
    fn test_day_iterator_empty_days() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz).unwrap();

        let days = collect_ok(calendar.days(start).take(2));

        assert!(days.iter().all(DayView::is_empty));
    }

    #[test]
    fn test_week_iterator_basic() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-05 12:00:00", tz).unwrap();

        let week = next_ok(calendar.weeks(start));

        assert_eq!(week.start_date(), chrono::NaiveDate::from_ymd_opt(2025, 11, 3).unwrap());
        assert_eq!(week.end_date(), chrono::NaiveDate::from_ymd_opt(2025, 11, 9).unwrap());
        assert_eq!(week.days().len(), 7);
    }

    #[test]
    fn test_week_iterator_event_count() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-05 12:00:00", tz).unwrap();

        let week = next_ok(calendar.weeks(start));

        assert_eq!(week.event_count(), 6);
        assert!(!week.is_empty());
    }

    #[test]
    fn test_owned_occurrence_fields() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-03 00:00:00", tz).unwrap();

        let day = next_ok(calendar.days(start));
        let occurrence = &day.events()[0];

        assert_eq!(occurrence.title(), "Planning");
        assert_eq!(occurrence.description(), Some("Weekly planning"));
        assert_eq!(occurrence.location.as_deref(), Some("Room A"));
        assert_eq!(occurrence.status, EventStatus::Confirmed);
        assert_eq!(occurrence.end_time(), occurrence.occurrence_time + Duration::hours(1));
    }

    #[test]
    fn test_day_view_helpers() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-04 00:00:00", tz).unwrap();

        let day = next_ok(calendar.days(start));

        assert_eq!(day.date(), chrono::NaiveDate::from_ymd_opt(2025, 11, 4).unwrap());
        assert_eq!(day.timezone(), tz);
        assert_eq!(day.event_count(), 1);
        assert!(!day.is_empty());
        assert_eq!(day.start().hour(), 0);
        assert_eq!(day.end().hour(), 0);
        assert_eq!(day.end().date_naive(), day.date().succ_opt().unwrap());
        assert_eq!(day.end_exclusive(), day.end());
        assert_eq!(day.end_inclusive().date_naive(), day.date());
    }

    #[test]
    fn test_cancelled_events_are_excluded_from_day_views() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-05 00:00:00", tz).unwrap();

        let day = next_ok(calendar.days(start));

        assert_eq!(day.event_count(), 1);
        assert!(day.events().iter().all(|event| event.status != EventStatus::Cancelled));
    }

    #[test]
    fn test_week_iterator_backward() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-12 09:00:00", tz).unwrap();

        let weeks = collect_ok(calendar.weeks_back(start).take(2));

        assert_eq!(weeks[0].start_date(), chrono::NaiveDate::from_ymd_opt(2025, 11, 10).unwrap());
        assert_eq!(weeks[1].start_date(), chrono::NaiveDate::from_ymd_opt(2025, 11, 3).unwrap());
        assert_eq!(
            weeks[0].days().iter().map(DayView::date).collect::<Vec<_>>(),
            vec![
                chrono::NaiveDate::from_ymd_opt(2025, 11, 10).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 11).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 12).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 13).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 14).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 15).unwrap(),
                chrono::NaiveDate::from_ymd_opt(2025, 11, 16).unwrap(),
            ]
        );
    }

    #[test]
    fn test_owned_occurrence_ordering_uses_start_time() {
        let mut calendar = Calendar::new("Ordering");
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        calendar.add_event(
            Event::builder()
                .title("Later")
                .start("2025-11-03 15:00:00", "America/New_York")
                .duration_minutes(30)
                .build()
                .unwrap(),
        );
        calendar.add_event(
            Event::builder()
                .title("Earlier")
                .start("2025-11-03 09:00:00", "America/New_York")
                .duration_minutes(30)
                .build()
                .unwrap(),
        );
        let start = timezone::parse_datetime_with_tz("2025-11-03 00:00:00", tz).unwrap();

        let mut events = next_ok(calendar.days(start)).events().to_vec();
        events.reverse();
        events.sort();

        assert_eq!(events[0].title(), "Earlier");
        assert_eq!(events[1].title(), "Later");
    }

    #[test]
    fn test_day_iterator_skip_to() {
        let calendar = Calendar::new("Skip");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz).unwrap();
        let mut iter = calendar.days(start);

        iter.skip_to(chrono::NaiveDate::from_ymd_opt(2025, 11, 5).unwrap());
        let day = next_ok(iter);

        assert_eq!(day.date(), chrono::NaiveDate::from_ymd_opt(2025, 11, 5).unwrap());
    }

    #[test]
    fn test_week_iterator_size_hint_and_skip_to() {
        let calendar = sample_calendar();
        let tz = timezone::parse_timezone("America/New_York").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-05 12:00:00", tz).unwrap();
        let mut iter = calendar.weeks(start);

        let (lower, upper) = iter.size_hint();
        assert!(lower > 0);
        assert_eq!(upper, Some(lower));
        assert_eq!(iter.len(), lower);

        iter.skip_to(chrono::NaiveDate::from_ymd_opt(2025, 11, 17).unwrap());
        let week = next_ok(iter);
        assert_eq!(week.start_date(), chrono::NaiveDate::from_ymd_opt(2025, 11, 17).unwrap());
    }

    #[test]
    fn test_day_iterator_size_hint_is_exact() {
        let calendar = Calendar::new("Hints");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz).unwrap();
        let iter = calendar.days(start);

        let (lower, upper) = iter.size_hint();
        assert!(lower > 0);
        assert_eq!(upper, Some(lower));
        assert_eq!(iter.len(), lower);
    }

    #[test]
    fn test_week_view_all_events_is_sorted_across_days() {
        let mut calendar = Calendar::new("All Events");
        let tz = timezone::parse_timezone("UTC").unwrap();

        calendar.add_event(
            Event::builder()
                .title("Wednesday")
                .start("2025-11-05 09:00:00", "UTC")
                .duration_minutes(30)
                .build()
                .unwrap(),
        );
        calendar.add_event(
            Event::builder()
                .title("Monday")
                .start("2025-11-03 09:00:00", "UTC")
                .duration_minutes(30)
                .build()
                .unwrap(),
        );

        let start = timezone::parse_datetime_with_tz("2025-11-03 00:00:00", tz).unwrap();
        let week = next_ok(calendar.weeks(start));
        let titles: Vec<_> = week.all_events().into_iter().map(|event| event.title()).collect();

        assert_eq!(titles, vec!["Monday", "Wednesday"]);
    }

    #[test]
    fn test_end_inclusive_zero_length_window_falls_back_to_start() {
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz).unwrap();
        let day = DayView::new(start.date_naive(), tz, start, start, vec![]);

        assert_eq!(day.end_inclusive(), start);
    }

    #[test]
    fn test_week_boundary_helpers_are_monday_aligned() {
        assert_eq!(first_full_week_start().weekday(), chrono::Weekday::Mon);
        assert_eq!(last_full_week_start().weekday(), chrono::Weekday::Mon);
        assert!(last_full_week_start().checked_add_days(Days::new(6)).is_some());
    }

    #[test]
    fn test_remaining_full_weeks_invalid_start_returns_zero() {
        let invalid_start = last_full_week_start().succ_opt().unwrap();
        assert_eq!(remaining_full_weeks(invalid_start, Direction::Forward), 0);
    }

    #[test]
    fn test_day_iterator_is_fused_after_exhaustion() {
        let calendar = Calendar::new("Fuse Day");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz).unwrap();
        let mut iter = calendar.days(start);

        iter.skip_to(chrono::NaiveDate::MAX);
        assert!(iter.next().unwrap().is_err());
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_week_iterator_is_fused_after_exhaustion() {
        let calendar = Calendar::new("Fuse Week");
        let tz = timezone::parse_timezone("UTC").unwrap();
        let start = timezone::parse_datetime_with_tz("2025-11-01 00:00:00", tz).unwrap();
        let mut iter = calendar.weeks(start);

        iter.skip_to(chrono::NaiveDate::MAX);
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
    }
}
