//! Event-tap plumbing between the engine and tap writers.
//!
//! **Status: milestone 2.** The tap *format* lives in `plause_events::TapEvent`;
//! this module owns how tap records get out of the engine:
//!
//! - **Offline:** the render loop hands `TapEvent`s straight to a writer.
//! - **Realtime (milestone 3):** the audio thread may not format strings or
//!   write files, so it pushes plain `TapEvent` structs into a pre-allocated
//!   ring buffer and a logger thread does the formatting and I/O.
//!
//! Both paths implement one `EventTap` trait so the engine core is identical
//! in either mode.
