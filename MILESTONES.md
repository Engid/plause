# Milestones

The detailed roadmap. Each milestone lists **deliverables**, **acceptance
criteria** (what must demonstrably work before the milestone is tagged), and
**explicit non-goals** (things deliberately deferred, so scope creep has to be
argued for in a PR rather than slipping in). The rules in
[ARCHITECTURE.md](ARCHITECTURE.md) apply to every milestone.

Status legend: `[x]` done · `[ ]` open

---

## v0.1 — Discovery & inspection

*Goal: point plause at a `.clap` and get a complete, trustworthy report — or a
diagnosis of exactly why not.*

### Deliverables

**`plause scan [dir]`**
- [x] Recursive scan for `.clap` entries (files on Linux/Windows, bundle
      directories on macOS; never descends into a bundle)
- [x] Default search paths per the CLAP spec (`CLAP_PATH` first, then the
      platform's user + system directories)
- [x] Helpful empty-result message listing the directories searched

**`plause-probe` test plugin**
- [x] New workspace crate, `cdylib`, built with clack-plugin; hermetic test
      subject for everything below (no external plugin needed to test plause)
- [x] Descriptor: id `org.plause.probe`, name, vendor, version, features
- [x] One stereo audio output port
- [x] One note input port and one note output port, CLAP dialect
- [x] At least two params with distinct ranges/defaults (one continuous, one
      stepped) so param reporting is meaningfully exercised
- [x] `process()` exists but only outputs silence (real behavior is v0.2)

**`plause_host::loader`**
- [x] Load a bundle via clack's `PluginEntry::load` (which resolves macOS
      bundle directories internally); bare `cargo build` dylibs accepted too,
      for dev loops
- [x] A dedicated error type (`LoadError`) where every failure mode is a
      distinct variant with an actionable message:
      - path does not exist
      - dynamic library failed to load / no `clap_entry` symbol (one variant —
        both arrive as the OS loader error, which is surfaced verbatim)
      - `clap_entry` present but null
      - macOS bundle binary resolution failed
      - entry reports an incompatible CLAP version (reports both versions)
      - `entry->init()` returned false
      - no plugin factory / factory lists zero plugins
- [ ] "Requested plugin id not present (list the ids that are)" — deferred:
      there is no id-targeted API yet; `inspect` reports all plugins in a
      bundle. Add alongside the first API that takes a plugin id (v0.2's
      `OfflineHost::load`)
- [x] Panics never cross the FFI boundary out of the plugin during inspection
      (`catch_unwind` around per-plugin instantiation + queries)

**`plause_host::instance`**
- [x] Instantiate each plugin in a bundle with a minimal host-handlers
      implementation (no-op `request_*` callbacks — they must not panic)
- [x] Main-thread extension queries producing an owned, serializable
      `PluginInfo`:
      - descriptor fields (id, name, vendor, version, description, features)
      - audio ports, both directions: id, name, channel count, port type,
        flags (main/…)
      - note ports, both directions: id, name, supported dialects, preferred
        dialect
      - params: id, name, module path, min/max/default, flags
        (automatable, stepped, hidden, …)
      - which standard extensions the plugin exposes (by extension id string;
        currently queried: audio-ports, note-ports, params, state, gui)

**`plause inspect <path> [--json]`**
- [x] Human-readable report (grouped: descriptor, extensions, ports, params)
- [x] `--json`: stable machine-readable schema (this is what tonarch CI will
      parse; treat field names as a public contract from day one)
- [x] Loader errors print as clean single-sentence diagnostics, not debug dumps
- [x] Non-zero exit code on any failure

**Integration tests (plause-host, `tests/inspect_probe.rs`)**
- [x] Test helper that `cargo build -p plause-probe`s and returns the cdylib
      path (platform-aware naming)
- [x] Happy path: inspect the probe, assert every field of `PluginInfo`
      matches what the probe declares; JSON shape spot-checked
- [x] Error paths: nonexistent path; a real file that is not a dylib; a real
      dylib without `clap_entry` (compiled on the fly with rustc — hermetic)

### Acceptance criteria

- [x] `plause inspect target/…/plause_probe.<dylib>` prints a complete,
      correct report on macOS
- [ ] …and on Linux (verify via the CI run after pushing; no local Linux box)
- [ ] `plause inspect` against nih-plug/nice-plug example plugins works
      headlessly (manual check, not CI — nice-plug examples need to be built
      first, e.g. from tonarch's vendored `vendor/nice-plug/examples`)
- [ ] Every loader error variant is hit by at least one test — still open for:
      `NullEntry`, `BundleResolveFailed`, `IncompatibleClapVersion`,
      `EntryInitFailed`, `NoPluginFactory`, `NoPlugins` (these need
      purpose-built misbehaving fixtures; fold into the probe's `misbehave`
      work in v0.4, or hand-craft tiny cdylibs like the no-entry test does)
- [x] `cargo test --workspace` green locally with no audio hardware and no
      display (CI confirmation pending push)

### Non-goals

- No `activate()`/`process()` (v0.2); instantiation + main-thread queries only
- No probe misbehavior variants (v0.2/v0.4)
- No GUI extension *hosting* — inspect only reports whether `clap.gui` exists

---

## v0.2 — Offline render & the event tap

*Goal: the killer feature. Deterministic, device-free processing with a
diffable record of everything that crossed the plugin boundary. After this
milestone, tonarch can snapshot-test its note-expression output in CI.*

### Deliverables

**`plause_host::engine` (the RT-safe zone becomes real)**
- [ ] Block-based process loop over clack's processor API: activate with the
      non-realtime flag, `start_processing`, N blocks, `stop_processing`,
      deactivate
- [ ] Event conversion both ways: `plause_events::Event` ↔ CLAP events
      (note on/off/end, note expression, param value), with sample-accurate
      offsets within blocks
- [ ] All buffers allocated at activation; the per-block path performs no
      allocation (enforced in tests via `assert_no_alloc`)
- [ ] Output events collected from every block with corrected absolute sample
      positions

**`plause_host::offline` — the `#[test]` API**
- [ ] `OfflineHost::load(path)?.activate(sample_rate, block_size)?`
- [ ] `send(TimedEvent)` / `send_fixture(&Fixture)` (seconds → samples here)
- [ ] `render_secs(f64) -> RenderOutput { audio: Vec<Vec<f32>>, events: Vec<TapEvent> }`
- [ ] Deterministic: same plugin + same fixture + same config ⇒ identical
      output events (document that audio determinism depends on the plugin)
- [ ] Simultaneous events are delivered in fixture order (stable sort)

**Tap output**
- [ ] `EventTap` trait in `plause_host::tap`; direct-writer impl for offline
- [ ] Input events logged with `→`, plugin output events with `←`, in one
      merged, sample-ordered stream

**CLI**
- [ ] `plause render --plugin X --events fixture.json --out out.wav --tap events.log --sample-rate N --block-size N`
- [ ] WAV written via a permissively-licensed writer (e.g. hound)
- [ ] `--tap -` writes the tap to stdout
- [ ] Malformed fixture JSON reports line/column and the failing event

**Probe growth (deterministic behaviors to assert against)**
- [ ] Emits a fixed-frequency, fixed-amplitude tone while ≥1 note is held
      (deterministic sample math, no random phase)
- [ ] Emits `note_end` exactly one block after `note_off`
- [ ] Echoes every received note-expression event to its note output port
- [ ] Param changes audibly/measurably affect output (gain param)

**Tests**
- [ ] Snapshot tests (insta) of tap output for a corpus of fixtures under
      `fixtures/`
- [ ] Audio assertions: RMS within expected bounds while note held, silence
      after `note_end`
- [ ] `assert_no_alloc` wraps the process loop in every engine test
- [ ] Tier-2 smoke tests against real plugins (nih-plug examples), `#[ignore]`
      by default, opt-in via `PLAUSE_TEST_PLUGIN=<path>`; a separate
      non-blocking CI job builds two nih-plug examples and runs them

### Acceptance criteria

- `plause render` on the probe produces byte-identical tap files across runs
  and across macOS/Linux CI
- A tonarch-shaped workflow works end to end: build a `.clap`, render a
  fixture, snapshot the tap
- Zero allocations detected in the block loop across the whole test suite

### Non-goals

- No realtime anything; no audio devices; no MIDI hardware
- No transport events (v0.5)
- No multi-plugin chains (v0.5)

---

## v0.3 — Realtime `run` & the dev loop

*Goal: `cargo xtask bundle && plause run --watch` is a plugin developer's
inner loop.*

### Deliverables

**Audio output**
- [ ] Internal `AudioBackend` trait (device enumeration, output stream with a
      callback receiving sample rate / buffer size); small on purpose
- [ ] cpal implementation (output only — duplex explicitly out of scope; see
      ARCHITECTURE.md "Dependencies" for the rtaudio plan)

**Realtime driver (the second driver of the same engine)**
- [ ] Audio callback drives the identical engine core used by offline
- [ ] Controller ↔ engine communication via pre-allocated ring buffers only:
      events in, tap records + notifications out
- [ ] Tap logger thread: drains the ring buffer, formats, writes; the audio
      thread never formats a string
- [ ] Graceful under/overrun behavior (count + report, never block)

**Input**
- [ ] Live MIDI input via midir, translated to note events
- [ ] Simple stdin note entry as a fallback (`n 60 on` style), so `run` is
      testable without MIDI hardware

**Dev loop**
- [ ] `plause run --plugin X --watch`: detect the `.clap` changing (mtime +
      content hash), deactivate, unload, reload, re-instantiate, reactivate
- [ ] Stretch: state preservation across reloads via the `clap.state`
      extension (save before unload, restore after)
- [ ] Host main-thread event loop: `request_callback` serviced; `clap.timer`
      support (stretch)
- [ ] Documented recipe for plugin repos (tonarch first): an xtask that
      bundles and execs plause

### Acceptance criteria

- `plause run` plays the probe audibly on macOS with live MIDI or stdin input
- `--watch` survives 50 consecutive rebuild/reload cycles without leaking
  instances or crashing (scripted soak test, can be manual/opt-in)
- The realtime driver passes the same engine test suite as the offline driver
  (minus device-dependent assertions)

### Non-goals

- No recording, no audio input, no duplex
- No plugin GUI windows (v0.4)

---

## v0.4 — GUI viewer & RT-safety checking

*Goal: eyes on the event stream, and the "help the whole ecosystem" feature.*

### Deliverables

**`plause-gui` (new crate, egui)**
- [ ] A *viewer* over `plause-host` — anything the GUI does must already be
      possible headlessly (ARCHITECTURE.md rule 2)
- [ ] Plugin picker fed by `discovery`
- [ ] Param panel: live values, editable, reflects plugin-side changes
- [ ] Live tap panel: scrolling event stream, direction-colored, pausable,
      with a filter box
- [ ] One clickable octave of keys sending note on/off (+ a note-expression
      slider bound to the held note — the tonarch-input test bed)
- [ ] Plugin GUI hosting in **floating-window mode only**

**RT checking (`--rt-check`)**
- [ ] `rt-check` cargo feature gating an rtsan-standalone integration:
      `process()` calls wrapped in nonblocking scopes
- [ ] Violations inside the *plugin under test* are caught and reported with
      the RTSan stack trace, prefixed by plause context (which block, which
      event was in flight)
- [ ] Probe gains a feature-flagged `misbehave` build that allocates and locks
      a mutex inside `process()`; a test proves `--rt-check` catches both
- [ ] CI job (Linux + macOS) running the rt-check suite
- [ ] README section: what RT violations are, what plause can and cannot catch
      (dynamic ≠ exhaustive; coverage depends on fixtures)

### Acceptance criteria

- A human can: pick the probe, play a note from the on-screen keyboard, watch
  note + expression events stream through the tap panel
- `plause render --rt-check` against the misbehaving probe fails with a stack
  trace pointing at the offending allocation; against the normal probe it
  passes

### Non-goals

- No embedded (reparented) plugin GUIs — floating only
- No waveform/piano-roll editing of any kind

---

## v0.5 — Chaining, transport & automation

*Goal: minimum DAW-shaped structure — enough to route tonarch's note
expression into a synth and hear/verify the result.*

### Deliverables

- [ ] Linear per-track chain: `[note source] → instrument → effect*`, N tracks
      summed to a master bus (a chain is the degenerate graph; a real graph
      crate is extracted only when a non-linear need exists)
- [ ] CLAP transport events in `process()`: tempo, play/stop, song position;
      fixtures gain an optional tempo + a `transport` event kind
- [ ] Param automation in fixtures: timed ramps
      (`{param, from, to, start, end, curve: linear}`) expanded to
      sample-accurate param events
- [ ] `plause render` accepts a session file (JSON) describing tracks/chains,
      not just a single plugin
- [ ] Tap gains a plugin-instance column so multi-plugin streams stay readable
      (append-only format change, per ARCHITECTURE.md rule 3)
- [ ] Candidate crate extractions reviewed: `plause-graph`, `plause-transport`
      (extract only if an external consumer exists)

### Acceptance criteria

- End-to-end tonarch scenario: tonarch (note source) → probe/any synth
  (instrument), tonarch's note-expression output verified in the tap *and*
  audible in the synth's output, offline and realtime
- Automation ramps produce sample-accurate param events verified by snapshot

### Non-goals (still)

- Recording, audio input, duplex streams
- Timeline/clip editing, project persistence beyond the session JSON
- Plugin sandboxing/out-of-process hosting (worth a design doc before v1.0,
  not before)

---

## Releases

- Each milestone ends with: CHANGELOG entry, annotated git tag, GitHub release
  with prebuilt `plause` binaries (macOS arm64 at minimum)
- **crates.io publishing is blocked** while clack is a git dependency
  (crates.io forbids git deps). `plause-events` has no clack dependency and
  can publish immediately; `plause-host`/`plause` publish when clack does.
  Until then: `cargo install --git https://github.com/Engid/plause plause`
