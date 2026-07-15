//! Loading a `.clap` bundle: dylib loading, `clap_entry` resolution, and
//! plugin factory enumeration.
//!
//! **Status: milestone 1.** This module is plause's error-reporting showcase —
//! every failure mode gets a distinct, actionable diagnostic:
//!
//! - path does not exist / is not a `.clap` bundle
//! - dynamic library fails to load (wrong architecture, missing deps)
//! - `clap_entry` symbol missing or null
//! - `clap_entry->init()` returns false
//! - factory missing or reports zero plugins
//! - panics crossing the FFI boundary (caught, never propagated)
