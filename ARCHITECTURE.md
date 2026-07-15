# Architecture

The rules in this document are load-bearing: they are what keeps a test
harness able to grow into a serious realtime host. Contributions are measured
against them.

## Crate map

```
crates/
├── plause-events/   # event model, JSON fixture format, tap format
│                    #   ← the crate plugin test suites depend on.
│                    #     serde-only, small, stable.
├── plause-host/     # the engine library — all hosting logic
└── plause/          # the CLI binary — a thin wrapper, no logic
```

`plause-host`'s internal modules mirror the crates this workspace may grow
into (`discovery`, `loader`, `instance`, `engine`, `offline`, `tap`, later
`realtime`/graph/transport). A module is extracted into a crate only when an
external consumer exists — not before.

**Dependency direction:** plause never depends on any plugin it tests.
Plugins' test suites depend on `plause-events` (and optionally `plause-host`)
as dev-dependencies. The `.clap` binary is the only interface between plause
and a plugin.

## Rule 1: engine/controller split (RT safety)

The engine core (`plause_host::engine`) is written **as if the audio thread
already exists**, even in milestones that only ever drive it from an offline
loop. Inside the RT-safe zone:

- No allocation. Buffers, routing tables, and event queues are allocated at
  activation time, never during processing.
- No locks, no syscalls, no I/O, no string formatting.
- Communication in and out of the zone goes exclusively through pre-allocated
  ring buffers / message passing. The controller side (loader, CLI, GUI)
  builds new state and swaps it in by message; it never mutates shared state
  under a lock.
- No panics across the plugin FFI boundary in either direction.

Offline rendering and realtime playback are **two drivers of the same core**:
`render` calls the process function from a plain loop; realtime mode calls it
from an audio callback. Neither driver may reach around the engine.

Enforcement, not just convention:

- CI wraps the offline process loop in
  [`assert_no_alloc`](https://github.com/Windfisch/rust-assert-no-alloc) —
  any allocation in the RT zone fails the build.
- A feature-gated [RTSan](https://github.com/realtime-sanitizer/rtsan-standalone-rs)
  build (`rt-check`) additionally catches locks and syscalls, including inside
  the plugin under test — exposed to users as `plause render --rt-check`.
- Both checks are dynamic: they only see executed paths. Rich fixtures are
  part of the safety story.

## Rule 2: CLI parity

Any capability not reachable headlessly does not exist. The GUI (milestone 4)
is a viewer over the same library API the CLI uses; "should the GUI do X?"
resolves to "does the CLI do X?".

## Rule 3: the tap format is append-only stable

`plause_events::TapEvent`'s display format is a snapshot-testing contract:
new event kinds may add new line shapes, existing line shapes never change.
In offline mode tap records go straight to a writer; in realtime mode the
audio thread pushes plain structs into a ring buffer and a logger thread does
all formatting and I/O (string formatting is not RT-safe).

## Rule 4: fixtures are seconds-based and code-first

Fixtures store timestamps in seconds so one fixture is meaningful at any
sample rate; the engine converts to sample offsets. JSON files are the
*serialization* of the builder API, not the primary interface — Rust test code
constructing events programmatically is the first-class path.

## Dependencies

- Permissive licenses only (MIT/Apache/ISC/BSD/zlib). Copyleft dependencies
  (GPL/LGPL/AGPL) are not accepted — this is why there is no VST3 support and
  why any future use of a JUCE-backed audio layer is off the table.
- `plause-events` stays serde-only.
- Audio device I/O (milestone 3) sits behind a small internal `AudioBackend`
  trait. cpal suffices for output-only monitoring; synchronized duplex
  (needed for any future recording) will use the `rtaudio` bindings or
  whatever backend proves out — the trait keeps that swappable.
