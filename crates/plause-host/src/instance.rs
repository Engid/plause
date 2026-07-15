//! Plugin instance lifecycle and extension negotiation, built on
//! [clack](https://github.com/prokopyl/clack)'s `clack-host`.
//!
//! **Status: milestone 1.** Owns the CLAP main-thread contract: create,
//! activate/deactivate, `request_callback` servicing, and querying which
//! extensions (note ports, params, audio ports, state, gui) a plugin supports —
//! the data behind `plause inspect`.
