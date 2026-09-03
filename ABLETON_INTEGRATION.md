# Using Modurust-Lite with Ableton Live via M4L and VST

## Overview

Modurust-Lite provides three interchangeable frontends for the headless `modurust_core` DSP engine:
1. **Standalone GUI** (`apps/standalone`) - Local control surface
2. **M4L Bridges** (`apps/m4l/*`) - Max for Live UDP control devices
3. **VST2 Plugin** (`vst/`) - VST instrument for Ableton Live

All frontends control the same DSP engine, allowing flexible integration with Ableton Live.

## Setting Up Ableton Live Integration

### Option 1: VST Plugin (Recommended for DSP Processing)
1. **Build the VST**: `cargo build -p modurust-vst --release`
2. **Locate the DLL**: `target/release/modurustvst.dll`
3. **Install in Ableton**:
   - Copy `modurustvst.dll` to your VST plugins folder
   - Rescan plugins in Ableton Live
   - Insert "Modurust Modurust" as an Instrument or Audio Effect track
4. **Features**:
   - 16 automatable parameters (filter, envelope, DSP, prime, NN, master)
   - MIDI input triggers the SuperInstancePool voice renderer
   - Stereo audio output from the voice pool
   - Low-latency processing via `CoreEngine::process_block`

### Option 2: M4L Bridges (Recommended for Control & Monitoring)
1. **Build the bridges**: `cargo run -p modurust_instrument` (etc.)
2. **Configure UDP ports**:
   - Ensure `config.json` matches bridge expectations:
     ```json
     {
       "udp_port": 9000,
       "osc_port": 9001,
       "status_port": 9002
     }
     ```
3. **Load in Ableton**:
   - Drag `ModurustLite_Instrument.amxd` onto a MIDI track
   - Drag `ModurustLite_Analyzer.amxd` onto an audio track (for monitoring)
   - Drag `ModurustLite_Modulator.amxd` onto a MIDI track (for modulation)
4. **Features**:
   - Instrument: Sends notes, prime/base, NN sensitivity, DSP params
   - Analyzer: Sweeps NN sensitivity, displays engine status
   - Modulator: Sweeps prime params, displays engine status
   - Bidirectional communication via UDP

### Option 3: Combined VST + M4L Approach
For maximum flexibility:
1. Load VST plugin on a MIDI instrument track
2. Place M4L Instrument bridge on same track (post-VST)
3. Place M4L Analyzer bridge on return track (monitoring output)
4. Use M4L bridges for visual control/monitoring while VST handles DSP

## Available Controls in Ableton

### Via VST Automation
All 16 parameters are automatable in Ableton's automation lanes:
- Filter Cutoff (20Hz-20kHz)
- Filter Resonance (0.1-10.0)
- Filter Mode (0=LP,1=HP,2=BP,3=Notch)
- Envelope Attack (0.001-5.0s)
- Envelope Release (0.001-5.0s)
- Waveform (0=Sine,1=Saw,2=Square,3=Triangle)
- Reverb Mix (0.0-1.0)
- Delay Time (0.01-2.0s)
- Delay Feedback (0.0-0.95)
- Prime Base (2-31)
- Prime Phase (0.0-1.0)
- NN Sensitivity (0.0-1.0)
- Instance Density (1-64)
- Instance Spread (0.0-1.0)
- Master Gain (0.0-2.0)

### Via M4L Bridges
M4L devices send/receive via UDP:
- **Instrument Bridge Sends**:
  - NoteOn/NoteOff (MIDI notes)
  - SetPrimeBase (value 2-31)
  - SetNnSensitivity (value 0.0-1.0)
  - SetInstanceDensity (value 1-64)
  - SetInstanceSpread (value 0.0-1.0)
  - SetMode (synth/sampler/effect/controller)
  - SetPrimePhase (value 0.0-1.0)
  - Graph text protocol commands (add/remove/connect nodes)
- **Instrument Bridge Receives**:
  - Labeled text status (mode, peak, voices, params)
- **Analyzer/Modulator Bridges**:
  - Similar send/receive patterns for their specific functions

### Graph Text Protocol (Live Modular Patching)
Send via M4L `udpsend` or automate via M4L message boxes:
```
graph_osc 1 sine 440        // Add sine oscillator at 440Hz
graph_filter 2 lowpass 1000 0.7 // Add LPF at 1kHz, Q=0.7
graph_connect 1 2           // Connect oscillator to filter
graph_output 2              // Set filter as output
graph_param 1 frequency 880 // Change oscillator frequency
graph_param 2 cutoff 500    // Change filter cutoff
graph_clear                 // Clear entire graph
```

## Adding Modular Features to Ableton Workflow

### 1. Modular Signal Chains
Use M4L to create and modify DSP chains in real-time:
- Create oscillator → filter → delay → reverb → output chains
- Dynamically change connection topology during performance
- Split signals to multiple processing paths using SplitterNode
- Route signals to multiple destinations using RouterNode

### 2. Parameter Automation & Modulation
- Automate any of the 16 VST parameters in Ableton's arrangement view
- Use LFONode to create rhythmic modulation (automate LFO frequency/depth)
- Use EnvelopeNode for complex envelope shapes beyond simple ADSR
- Use multiple LFOs/envelopes for complex modulation routing

### 3. Preset Management
Implement via M4L or external controller:
- Save current graph configuration + parameter values to file
- Load presets to instantly change sounds
- Create banks of presets for different song sections
- MIDI program change messages to switch presets

### 4. Performance Features
- Use XY morph pad (via standalone or custom M4L) for simultaneous prime/NN control
- Map Ableton's Macro controls to multiple parameters simultaneously
- Use Ableton's Follow Actions to automate preset changes
- Create drum racks where each cell is a different Modurust preset

### 5. Hardware Integration Pathway
For eventual modular hardware deployment:
- **Phase 1**: Use Ableton + M4L/VST to design and test patches
- **Phase 2**: Export patch configuration (graph + parameters)
- **Phase 3**: Load configuration onto embedded device running minimal modurust_core
- **Phase 4**: Hardware controls (knobs, switches, CV) map to engine parameters
- **Phase 5**: Audio output via hardware DAC/I2S

## Technical Notes

### Latency
- VST2 plugin: ~5-10ms round-trip (depends on buffer size)
- M4L UDP: ~1-2ms latency for control messages
- Audio processing: <1ms via `CoreEngine::process_block`

### Reliability
- UDP is fire-and-forget but sufficient for control data
- Status feedback enables bidirectional verification
- VST plugin uses direct engine calls (no networking overhead)
- All frontends share the same engine state when connected to same instance

### Extensibility
- Add new DSP nodes in `core/src/dsp/` and expose via `EngineCommand::GraphAddNode`
- Add new parameters to `Param` enum and implement in `apply_param_to_engine`
- Add new UDP commands in `core/src/comm/` for hardware-specific controls
- Add new M4L controls by extending the bridge applications

## Troubleshooting

### VST Not Loading in Ableton
- Ensure you built the **release** version (`--release`)
- Check that Ableton scans the correct VST folder
- Verify you're using VST2 (not VST3) - Ableton supports both
- Check Windows Defender/Antivirus isn't blocking the DLL

### M4L Not Communicating
- Verify UDP ports in `config.json` match bridge expectations
- Check Windows Firewall allows UDP traffic on ports 9000-9005
- Ensure only one application binds to UDP 9002 (text status feed)
- Use `netstat -ano | findstr :9000` to check port usage

### No Audio Output
- Verify audio engine is started (standalone: "Start Audio" button)
- Check VST plugin is receiving MIDI notes (test with Ableton's MIDI clip)
- Ensure audio output device is correctly selected in system/audio settings
- Test with standalone GUI first to isolate engine vs VST issue

---

**Ready for Ableton Integration**: 
1. Build VST: `cargo build -p modurust-vst --release`
2. Copy `target/release/modurustvst.dll` to VST folder
3. Load in Ableton as instrument
4. Use M4L bridges for control/monitoring
5. Create modular patches via graph text protocol
6. Automate parameters in Ableton's arrangement view
