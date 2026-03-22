# Eventix Technical Roadmap

This document outlines the strategic technical direction for the `eventix` crate. It serves as the single source of truth for feature prioritization, ensuring alignment with enterprise-grade scheduling requirements and modern AI-driven ecosystems.

## 1. Core Platform Logic (Booking Lifecycle)
*Objective: Transition from a pure calendar parser to a stateful booking engine.*

- [x] **Booking State Machine** `(Priority: High)`
    - **RFC**: Introduce `EventStatus` enum (`Confirmed`, `Tentative`, `Cancelled`, `Blocked`).
    - **Implementation**: strict state transitions (`confirm()`, `cancel()`, `reschedule()`) to ensure data integrity.
    - **Impact**: Enables `gap_validation` to automatically ignore cancelled events, significantly reducing implementation complexity for consumers.

- [x] **Advanced Recurrence Optimization** `(Priority: Medium)`
    - **Optimization**: Lazy `OccurrenceIterator` computes occurrences on demand, zero upfront allocation for infinite recurrence rules.
    - **Feature**: `occurrences_between()` uses windowed expansion (`take_while` + `filter`) to only compute instances within the relevant view window.
    - **DST safety**: `resolve_local()` handles spring-forward gaps via pre-gap UTC offset; `intended_time` parameter prevents wall-clock drift across DST transitions.

## 2. API & Integration Ecosystem
*Objective: Enable seamless integration with modern web stacks and AI agents.*

- [ ] **AI & Natural Language Support** `(Priority: Medium)`
    - **Feature**: Integrate `chrono-english` (behind `feature = "nlp"`) to parse inputs like *"Next Tuesday at 2pm"*.
    - **Use Case**: Critical for Chatbots and AI Agents to directly generate `Event` structs from user prompts.

- [ ] **Model Context Protocol (MCP) Adapter** `(Priority: Low)`
    - **Example**: Create `examples/mcp_server.rs` demonstrating `eventix` as an MCP Resource/Tool.
    - **Value**: Allows AI models (Claude, Gemini) to natively "read" and "book" slots via `eventix` logic.

- [ ] **Async Runtime Support** `(Priority: medium)`
    - **Feature**: `async` variants for compute-heavy operations (`find_gaps_async`).
    - **Target**: High-throughput `tokio` (Axum/Actix) server environments.

## 3. Data Persistence & Interoperability
*Objective: Standardize data storage and exchange.*

- [ ] **Persistence Traits** `(Priority: Medium)`
    - **Design**: Define `CalendarStore` and `EventRepository` traits.
    - **Goal**: Decouple logic from storage, allowing users to plug in `SQLx`, `Diesel`, or In-Memory backends.

- [ ] **Modern Exchange Formats** `(Priority: Low)`
    - **Standard**: Implement **JSCalendar (RFC 8984)** support.
    - **Benefit**: JSON-native format is superior for REST/GraphQL APIs compared to legacy `.ics`.

## 4. Quality Assurance & Reliability
*Objective: Maintain zero-defect reliability for critical scheduling data.*

- [ ] **Property-Based Testing** `(Priority: High)`
    - **Action**: Implement `proptest` suites for `gap_validation`.
    - **Goal**: mathematically guarantee no overlapping slots are missed under edge-case conditions (e.g., DST transitions).

## 5. Calendar View API
*Objective: Expose lazy, UI-friendly calendar traversal primitives for day/week rendering.*

- [x] **Day and Week View Iterators** `(Priority: Medium)`
    - **Feature**: Add lazy `Calendar::days()` / `days_back()` and `Calendar::weeks()` / `weeks_back()` iterators that yield pre-bucketed `DayView` and `WeekView` values.
    - **Use Case**: Lets consumers render personal calendar UIs in frameworks like Yew, Leptos, and Dioxus without eagerly expanding wide date ranges or manually grouping occurrences by day.

## 6. v0.5.1 API Polish
*Objective: Small additive improvements requested by early users and follow-up review feedback.*

- [ ] **Calendar `FromStr` parsing** `(Priority: Medium)`
    - **Feature**: Parse ICS data directly from `&str` so callers can load calendar payloads without a temporary file.
    - **Value**: Useful for tests, embedded fixtures, and API handlers that already have the raw string body.

- [ ] **Streaming ICS import** `(Priority: Medium)`
    - **Feature**: Add `Calendar::from_ics_reader(impl Read)` for incremental loading from files, sockets, or database-backed readers.
    - **Value**: Avoids forcing all ICS payloads into memory up front.

- [ ] **DayView date conversions** `(Priority: Low)`
    - **Feature**: Implement `From<DayView> for NaiveDate` and `From<&DayView> for NaiveDate`.
    - **Value**: Makes calendar views easier to feed into UI and serialization layers.

- [ ] **Yew ergonomics docs** `(Priority: Low)`
    - **Feature**: Document the `Rc<DayView>` wrapping pattern for component props.
    - **Value**: Clarifies the intended usage for UI users without changing runtime behavior.

- [ ] **Optional serde for views** `(Priority: Low)`
    - **Feature**: Gate `DayView` and `WeekView` serialization behind an opt-in `serde` feature.
    - **Value**: Helps SSR and persistence consumers without imposing serde on every downstream build.

## 7. v0.6.0 Performance / Bigger Features
*Objective: Scale out the calendar view layer and import path for heavier workloads.*

- [ ] **K-way merge optimization** `(Priority: Medium)`
    - **Feature**: Merge per-event occurrence streams instead of materializing every event's range independently.
    - **Value**: Reduces work when rendering large day/week windows across many recurring events.

- [ ] **Chunked/lazy ICS loading** `(Priority: Low)`
    - **Feature**: Stream ICS parsing in chunks so enterprise-scale calendars do not need to fit fully in memory.
    - **Value**: Better for very large imports, long-running services, and database-backed sync pipelines.

- [ ] **Configurable week start** `(Priority: Low)`
    - **Feature**: Allow Sunday-first calendars for regions that expect that convention.
    - **Value**: Improves UX for US-style calendar rendering without changing the default ISO behavior.

- [ ] **MonthView / YearView iterators** `(Priority: Low)`
    - **Feature**: Extend the lazy view model beyond day/week traversal.
    - **Value**: Useful for full calendar grids and high-level overview screens.
