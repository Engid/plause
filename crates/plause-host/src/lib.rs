//! The plause hosting engine: everything the `plause` CLI (and eventually the
//! GUI) does lives in this library. The binaries are thin wrappers — if a
//! capability isn't reachable through this crate, it doesn't exist.
//!
//! # Module map
//!
//! The modules mirror the crate splits this workspace may grow into (see
//! `ARCHITECTURE.md`), so boundaries stay clean while everything ships as one
//! crate:
//!
//! | Module        | Role                                                    | Status |
//! |---------------|---------------------------------------------------------|--------|
//! | [`discovery`] | Find `.clap` bundles on disk                            | done   |
//! | [`loader`]    | Load bundles, resolve `clap_entry`, report failures well | done   |
//! | [`instance`]  | Instantiation + extension queries → [`instance::BundleInfo`] | done |
//! | [`engine`]    | **RT-safe** process core: buffers, event routing        | milestone 2 |
//! | [`offline`]   | Device-free rendering; the `#[test]`-friendly API       | milestone 2 |
//! | [`tap`]       | Event-tap plumbing between engine and writers           | milestone 2 |
//!
//! Realtime playback (`AudioBackend` trait + device output) is milestone 3 and
//! intentionally has no module yet.

pub mod discovery;
pub mod engine;
pub mod instance;
pub mod loader;
pub mod offline;
pub mod tap;
