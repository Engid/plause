//! Event vocabulary for the plause CLAP test host.
//!
//! This crate defines the three data shapes everything else in plause speaks:
//!
//! - [`Event`] / [`TimedEvent`] — the events that cross a plugin boundary
//!   (notes, note expression, parameter changes), with a builder-style API so
//!   test code reads naturally:
//!
//!   ```
//!   use plause_events::{Event, Fixture};
//!
//!   let fixture = Fixture::new()
//!       .event(Event::note_on(0, 60, 1, 0.8).at_secs(0.0))
//!       .event(Event::tuning(1, 0.31).at_secs(0.1))
//!       .event(Event::note_off(0, 60, 1).at_secs(2.0));
//!   # assert_eq!(fixture.events.len(), 3);
//!   ```
//!
//! - [`Fixture`] — a JSON-serializable sequence of timed input events, so the
//!   same sequences drive `plause render` on the command line and `#[test]`
//!   functions in a plugin's own test suite.
//!
//! - [`TapEvent`] — one line of the event tap: a sample-timestamped,
//!   direction-tagged record of an event crossing the plugin boundary, with a
//!   stable text format designed for snapshot diffing.
//!
//! It intentionally has no audio, threading, or CLAP dependencies: plugin test
//! suites depend on this crate, and it must stay small and stable.

mod event;
mod fixture;
mod tap;

pub use event::{Event, ExpressionKind, TimedEvent};
pub use fixture::Fixture;
pub use tap::{Direction, TapEvent};
