//! The realtime-safe process core.
//!
//! **Status: milestone 2.** This module is the RT-SAFE ZONE (see
//! `ARCHITECTURE.md`). Code here is written as if the audio thread already
//! exists, even though milestone 2 only drives it from an offline loop:
//!
//! - **No allocation.** All buffers and routing structures are allocated at
//!   activation time. CI wraps the process loop in `assert_no_alloc`.
//! - **No locks, no syscalls, no I/O.** Communication in and out goes through
//!   pre-allocated ring buffers only.
//! - **No panics across the boundary.** Plugin calls are isolated.
//!
//! Offline rendering ([`crate::offline`]) and realtime playback (milestone 3)
//! are two drivers of this same core; neither may reach around it.
