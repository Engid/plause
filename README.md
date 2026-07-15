# plause

**A headless CLAP host for testing plugins.** Load a `.clap`, feed it notes,
note expression, and parameter events, and inspect everything that comes back —
sample-accurate, deterministic, no audio interface required.

> testing CLAP plugins deserves ap*plause*

## Why

Testing a CLAP plugin today means loading it into a full DAW and listening.
plause makes the plugin boundary observable and scriptable instead:

- **Test plugins in `cargo test`.** The engine is a library; the CLI is a thin
  wrapper. Your plugin's test suite can load the compiled `.clap`, send it
  events, and assert on the events and audio that come back — through the real
  plugin ABI, not an in-process shortcut.
- **Device-free by design.** CLAP plugins never touch audio hardware — the host
  owns the buffers. plause's core is offline rendering: deterministic output
  that runs on any CI box. A realtime audio device is an optional sink, not a
  requirement.
- **A diffable event tap.** Every event crossing the plugin boundary, in both
  directions, logged with sample timestamps in a stable line format built for
  snapshot testing:

  ```text
         480 → note_on    ch=0 key=60 id=1 vel=0.800
        4800 ← note_expr  id=1 kind=tuning val=0.3100
       96000 ← note_end   ch=0 key=60 id=1
  ```

## Usage (target CLI)

```sh
plause scan                                  # find installed .clap bundles
plause inspect my-plugin.clap [--json]       # descriptors, extensions, ports, params
plause render --plugin my-plugin.clap \
    --events fixture.json \
    --out out.wav --tap events.log           # offline render + event tap
plause run --plugin my-plugin.clap --watch   # realtime, hot-reload on rebuild
```

## Status

Early scaffold. `plause scan` works; everything else is roadmap:

1. **v0.1** — `scan` + `inspect` with excellent error reporting
2. **v0.2** — `render`: offline engine, JSON fixtures, event tap, snapshot tests
3. **v0.3** — `run`: realtime output, MIDI in, `--watch` hot reload
4. **v0.4** — minimal egui viewer; `--rt-check` (RTSan) realtime-safety checking
5. **v0.5** — plugin chaining, transport, automation

See [ARCHITECTURE.md](ARCHITECTURE.md) for the design rules that keep the
"maybe one day a DAW" door open — most importantly, the process core is
realtime-safe from day one even though the first milestones never spawn an
audio thread.

## Scope

- **CLAP only.** No VST3: it would drag in Steinberg's GPL/proprietary dual
  license, and CLAP is the format that deserves better tooling.
- **Not a DAW (yet).** No timeline editing, no microtonal anything — plause
  stays a _generic_ host precisely so it's a fair test of what generic hosts
  see from your plugin.
- Built on [clack](https://github.com/prokopyl/clack).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. All dependencies are permissively
licensed; copyleft dependencies are not accepted (see ARCHITECTURE.md).
