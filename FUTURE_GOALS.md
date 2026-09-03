# Modurust-Lite - Future Goals for Modular DSP Engine

## Current State

**Modurust-Lite is a headless modular DSP audio engine** with three interchangeable frontends:
- Standalone egui GUI (`apps/standalone`)
- Max for Live UDP bridges (`apps/m4l/*`)
- VST2 plugin (`vst/`)

**There is NO DAW, NO timeline, NO clip editing, NO automation system, NO transport controls.**

This is a modular synthesizer engine that can run:
- Standalone on desktop
- Inside Ableton Live as a VST2 instrument
- Controlled via M4L devices using UDP
- Eventually on embedded/hardware platforms

## Future Goals

### Phase 1: VST Plugin Improvements (2-4 weeks)

**VST2 Stability**
- [ ] Add OSC listener to VST for OSC-compatible controllers
- [ ] Proper sample rate handling for different buffer sizes
- [ ] Parameter smoothing/interpolation
- [ ] Preset serialization (VST banks/program change)

**VST3 Migration Path**
- [ ] Evaluate `vst3` crate or `hpp` for VST3 support
- [ ] VST3 parameter handling (IDs, normalized values)
- [ ] VST3 sidechain support (audio input processing)
- [ ] VST3 editor GUI support

**M4L + VST Integration**
- [ ] Full bidirectional control: M4L → VST status feedback
- [ ] M4L visual control panel for all 16 parameters
- [ ] M4L XY morph pad for prime/NN parameters
- [ ] M4L graph patch commands (add/connect/remove nodes via UDP)

### Phase 2: DSP Engine Enhancements (1-3 months)

**Filter Improvements**
- [ ] State-variable filter (4-mode simultaneous output)
- [ ] Ladder filter (Moog-style resonance)
- [ ] Formant filter (vocal synthesis)
- [ ] FFT-based spectral filters

**Effect Nodes**
- [ ] Chorus, Phaser, Flanger nodes
- [ ] Compressor, Gate, Limiter nodes
- [ ] Convolution reverb with impulse responses
- [ ] Granular synthesis node

**Oscillator Improvements**
- [ ] Wavetable oscillator with custom tables
- [ ] FM synthesis support
- [ ] Physical modeling strings (Karplus-Strong)
- [ ] Noise generator with filtering options

**Neural Network**
- [ ] ONNX model loading at runtime
- [ ] Latent space interpolation UI
- [ ] Parameter modulation via NN

### Phase 3: Graph System & Control (2-4 months)

**Live Modular Patching**
- [ ] Full `graph_*` command support from M4L
- [ ] Node parameter modulation
- [ ] LFO destinations for all parameters
- [ ] Envelope follower/control voltage generation

**Preset System**
- [ ] Save/load complete node graphs
- [ ] Preset banks with categories
- [ ] Import/export preset files
- [ ] Random preset generator

**MIDI Enhancement**
- [ ] More MIDI message types (CC, Pitch Bend, Aftertouch)
- [ ] MIDI clock synchronization
- [ ] MIDI Program Change mapping
- [ ] MPE support (if needed)

### Phase 4: Hardware Deployment (3-6 months)

**Embedded Build**
- [ ] Remove desktop dependencies (egui, bevy, std::net)
- [ ] Minimal build with just core DSP
- [ ] I2S audio codec integration
- [ ] MIDI UART interface
- [ ] GPIO control knobs/buttons

**Target Platforms**
- [ ] Raspberry Pi (audiohat, USB audio)
- [ ] Teensy/Arduino with audio shields
- [ ] ESP32 with I2S and WiFi control

**Hardware Features**
- [ ] CV control voltage output
- [ ] Gate/trigger output
- [ ] Local knob/fader control
- [ ] OLED/display for feedback

### Phase 5: Ableton Integration (3-6 months)

**Enhanced M4L Devices**
- [ ] Visual node editor interface
- [ ] Real-time oscilloscope
- [ ] Spectrum analyzer
- [ ] Parameter morph controls
- [ ] Preset browser within M4L

**Ableton Workflow**
- [ ] MIDI Learn functionality
- [ ] Parameter linking to Ableton automation
- [ ] Macro control mapping
- [ ] Follow action-style preset changes

**Performance Features**
- [ ] Low-latency optimization (<10ms)
- [ ] CPU usage monitoring
- [ ] Voice counter display
- [ ] Polyphony limiting controls

## Implementation Priority

| Priority | Feature | ETA | Notes |
|----------|---------|-----|-------|
| 1 | VST3 support | 4-6 weeks | Critical for modern DAW compatibility |
| 1 | M4L bidirectional control | 2-3 weeks | Enables full control/monitoring |
| 2 | Filter expansions | 4-8 weeks | More synthesis options |
| 2 | Effects chain nodes | 4-8 weeks | Reverb, delay, chorus |
| 2 | Live graph patching | 6-10 weeks | Core modular workflow |
| 3 | Hardware embed | 12-16 weeks | Requires significant refactoring |
| 3 | Preset system | 4-6 weeks | User retention feature |

## Technical Notes

### Current Dependencies
- `vst` 0.4 (VST2 - deprecated but works)
- `cpal` 0.15 (audio output)
- `midir` (MIDI input)
- `crossbeam-channel` (threading/sync)
- `eframe` 0.26 (standalone GUI)

### Build System
```bash
# Check all
cargo check --workspace

# Run tests
cargo test --workspace

# Build VST release
cargo build -p modurust-vst --release

# Build all
cargo build --release
```

### Test Coverage
- 47 tests passing (20 unit + 20 integration + 7 VST)
- Voice pool polyphony tested
- UDP/OSC protocol tested
- Graph operations tested
- Parameter handling tested

## Conclusion

Modurust-Lite is a modular DSP engine - not a DAW. Future work focuses on:
1. Making the VST plugin professional-grade (VST3, presets, M4L integration)
2. Expanding DSP capabilities (filters, effects, oscillators)
3. Enabling hardware/embedded deployment

No timeline views, clip matrix, automation, or traditional DAW features will be added - that's not what this project is about.
