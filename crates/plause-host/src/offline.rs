//! Device-free offline rendering — the heart of plause as a *test* host, and
//! the API plugin test suites call directly.
//!
//! **Status: milestone 2.** CLAP plugins never touch audio hardware: the host
//! owns the buffers and calls `process()`. Offline mode exploits that — no
//! audio device, deterministic output, runs on any CI box. The target API:
//!
//! ```ignore
//! let mut host = OfflineHost::load("target/bundled/tonarch.clap")?
//!     .activate(48_000.0, 256)?;
//!
//! host.send(Event::note_on(0, 60, 1, 0.8).at_secs(0.0));
//! host.send(Event::tuning(1, 0.31).at_secs(0.1));
//!
//! let out = host.render_secs(2.0)?;
//! assert!(out.events.iter().any(|e| e.event.is_note_expression()));
//! ```
//!
//! Plugins are activated with CLAP's non-realtime flag so well-behaved plugins
//! know not to rely on wall-clock time.
