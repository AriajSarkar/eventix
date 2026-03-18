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
