# AGENTS.md

## Setup commands

Install Rust toolchain:
```bash
rustup update stable
cargo build --workspace
```

Build commands:
```bash
# Check the workspace (single rustc process)
cargo check --workspace

# Build all targets
cargo build --workspace

# Build release versions
cargo build --workspace --release

# Run all tests
cargo test --workspace
```

## Rust compilation constraint

**CRITICAL:** This project is configured to never use more than one rustc process at a time. This is enforced via `.cargo/config.toml` with `jobs = 1` under `[build]`. Do not override this with `--jobs` or `-j` flags as it can cause race conditions with audio engine initialization.

## Project overview

Modurust-Lite is a headless, modular DSP audio engine in Rust, exposed via:
- **Standalone GUI** (`apps/standalone`) — migrating to **iced** (retrained-mode declarative GUI)
- **M4L bridges** (`apps/m4l/*`) — Max for Live UDP control devices  
- **VST2 plugin** (`vst/`) — VST instrument for DAWs (using `vst` 0.4 crate)
- **CLAP plugin** (`clap/`) — CLAP plugin for modern DAWs (using `clap-sys` 0.3)

Workspace structure:
```
modurust-lite/
├── Cargo.toml           workspace root
├── .cargo/config.toml   build.jobs = 1 (single rustc process)
├── core/                 modurust_core (headless engine)
├── apps/standalone/      **iced** GUI (migrated from egui)
├── apps/m4l/            instrument, analyzer, modulator
├── plugins/             plugin implementations
│   ├── vst/             VST2 plugin (vst 0.4)
│   └── clap/             CLAP plugin (clap-sys 0.3)
└── docs/                architecture, roadmap, node features, strategic-roadmap
```

## Workspace members

| Package | Description |
|---------|-------------|
| `modurust_core` | Headless DSP engine with voice pool, prime scheduler, NN |
| `modurust_standalone` | egui-based standalone application (migrating to iced) |
| `modurust-vst` | VST2 plugin wrapper |
| `modurust-clap` | CLAP plugin wrapper |
| `m4l_instrument` | Max for Live instrument bridge |
| `m4l_analyzer` | Max for Live analyzer bridge |
| `m4l_modulator` | Max for Live modulator bridge |

## Code style

- Standard Rust idioms with `rustfmt`
- `clap` crate for CLI argument parsing
- **GUI uses `iced`** (retrained-mode declarative GUI) — migrate away from egui/eframe (currently egui-based, see `apps/standalone`)
- No foreign function interfaces or unsafe code in public APIs
- DSP nodes implement the `DspNode` trait pattern
- Use `num_complex::Complex` for complex-valued signals
- Prefer `crossbeam_channel` for inter-component communication

## Testing instructions

Run tests via cargo:
```bash
# Run all 47 tests across the workspace
cargo test --workspace

# Run specific package tests
cargo test -p modurust_core

# Run specific test file
cargo test -p modurust_core --test engine_comm
```

Integration tests verify:
- Voice-pool polyphony rendering
- UDP binary and text protocol
- OSC decoding
- Status packet encoding
- Config hot-reload
- Graph text protocol commands

## Common development workflows

```bash
# Run a core example (audio engine demo)
cargo run -p modurust_core --example engine_demo

# Run the standalone GUI
cargo run -p modurust_standalone

# Build plugins for deployment
cargo build -p modurust-vst --release
cargo build -p modurust-clap --release

# Quick check during development
cargo check -p modurust_core
```

## Protocol documentation

Engine status and control via UDP:
- Port 9000: Binary control commands (NoteOn/NoteOff, SetPrimeBase, etc.)
- Port 9001: OSC fallback (`/note_on`, `/note_off`, `/mode`, etc.)
- Port 9002: Text status feed
- Ports 9003-9005: Binary status packets

## Build configuration

The `.cargo/config.toml` enforces:
- `jobs = 1` — Single rustc process only
- Aliases: `dev`, `devr`, `devc`, `devt`, `devcc` for common workflows

## Research Patterns (from deep-dive studies, Oct 2026)

### Drum Machine engine patterns (from autokit source study)
- **Spectral analysis** (`analysis/features.rs`): attack_time (start→peak), decay_time (peak→−20dB), spectral_centroid Hz, spectral_flatness, sub_energy_ratio (<120 Hz), high_freq_ratio (>4 kHz), peak, duration, is_percussive. Single FFT pass computes all spectral stats.
- **Sample library** (`analysis/library.rs`): background thread scan, LibraryCache skips DSP analysis for unchanged files, MAX_DURATION_SECS = 4.0 filter, Arc-shared mono f32 audio data.
- **Kit** (`engine/kit.rs`): SampleCategory enum (Kick, Snare, Hihat, Clap, Tom, Perc, Cymbal, Bass, Synth, Other), NUM_PADS = 8, GM drum map midi notes 36+.

### Drum sequencer patterns (from autokit source study)
- `ConditionTrig` {Always, Every(N), NotEvery(N), Fill, NotFill} — Elektron-style. `Step` plocks: velocity/probability/pan/pitch/fx_rvb/fx_dly/fx_filter (None = inherit).
- `Lane` per pad, 16 steps, muted/solo, per-pattern fx sends. `PatternBank` = 16 patterns, active/queued.
- `MasterAutomation` = per-pattern per-step FX arrays (reverb_mix/delay_mix/dj_filter as Option). `MasterFxBase` = captured per-pattern base values.

### Master FX bus patterns (from autokit source study)
- 4-bus send architecture: voices → dry_bypass → dj_filter → master; send_rvb → reverb; send_dly → delay. Bipolar SVF DJ filter (−1=LP kill, +1=HP kill, 0=bypass). True sends; filter takes wet tails.
- Delay divisions: {1/32, 1/16, 1/8, 1/4} of a beat.

### Modular synth engine patterns (from rust-synth source study)
- FunDSP "hacker" graph: typed closure-based DSP graph, `Net` rebuilt wholesale on preset swap; rebuild is cheap enough for live use.
- `EngineHandle`: pre-allocated track slots (MAX_TRACKS=8), `Arc<Mutex<Net>>` master graph, `rebuild_graph()`.
- `Genome` = {freq, cutoff, resonance, reverb_mix, pulse_depth, pattern_hits, pattern_rotation, character}; mutate perturbs + freq snaps to golden pentatonic lattice; crossover mixes genes.

## Security considerations

- No network traffic beyond UDP control/status packets (user-configurable ports)
- All audio/MIDI processing is local with no external data sources
- ONNX model loading is feature-gated behind `nn` feature flag