# Modurust-Lite Execution TODO

## Phase H — Genuine Functional Fixes (2026-01-08, commits c33f755 + 38e2617)
- [x] H1: CoreEngine renders the SuperInstancePool polyphonically
- [x] H2: Master ADSR gate keyed on first/last active voice; prime-gate ducking; NN morph drives filter cutoff.
- [x] H3: `EngineCommand::AllNotesOff` (proper all-notes-off; not SetInstanceDensity(0)).
- [x] H4: `CpalEngineHost` retains the command sender (was dropped); mono scratch buffer reused (no per-frame allocs).
- [x] H5: OSC fallback extended to all 7 commands; `/mode` String-vs-&str comparison fixed.
- [x] H6: Standalone GUI starts cpal audio (▶ Start Audio), note buttons drive the voice pool, All Notes Off button, config→engine live push.
- [x] H7: New integration test `test_engine_voice_pool_renders_polyphony` (30 total).
- [x] H8: Docs updated (README, changes-features-ACCURATE, ROADMAP, node-system-features, frontend-status, architecture).
- [x] H9: Pushed to origin/master.

## Phase A — Config Hot-Reload End-to-End
- [x] A1–A3: Config hot-reload, engine wiring, GUI config watcher.

## Phase B — Full Modular DSP Engine
- [x] B1–B4: Full DSP node library, extra_nodes, graph, voice pool.

## Phase C — Audio Backend (cpal)
- [x] C1: Real cpal output stream.

## Phase D — MIDI Backend (midir/cpal)
- [x] D1: MIDI input abstraction.

## Phase E — Prime Scheduler & Complex NN
- [x] E1–E2: Prime patterns, complex NN, ONNX loader.

## Phase F — Comm & M4L Bridges
- [x] F1–F2: UDP/OSC bridge, M4L bridges.

## Phase G — Tests, Examples, Docs, Git Hygiene
- [x] G1–G5: 37 tests, examples, docs, initial commit.

## Phase I — Graph Integration for Extra Nodes (2026-01-08)
- [x] I1: Add `DspNode` impl for `MixerNode`
- [x] I2: Add `DspNode` impl for `SplitterNode`
- [x] I3: Add `DspNode` impl for `StereoDelayNode`
- [x] I4: Add `GraphNodeKind::Mixer/Splitter/StereoDelay` + `NodeKind` variants
- [x] I5: Wire into `add_node_at`, `add_*`, `apply_param`, `GraphParam`
- [x] I6: Add unit tests for Mixer, Splitter, StereoDelay in graph

## Phase J — Text Protocol Graph Commands
- [x] J1: Add `graph_add`, `graph_remove`, `graph_connect`, `graph_disconnect`, `graph_clear` text commands
- [x] J2: Map text commands to `EngineCommand::Graph*`

## Phase K — Interactive M4L Bridges
- [x] K1: `instrument/src/main.rs` — interactive run-loop with chord progression + status listening
- [x] K2: `analyzer/src/main.rs` — interactive run-loop with NN sweep + status listening
- [x] K3: `modulator/src/main.rs` — interactive run-loop with prime sweep + status listening

## Phase L — Tests & Docs
- [x] L1: Integration tests for graph text protocol commands
- [x] L2: Update docs (README, architecture, changes-features-ACCURATE, node-system-features, frontend-status)
- [x] L3: Final `cargo test --workspace` verification (47 tests: 27 unit + 20 integration) + commit + push
