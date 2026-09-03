use crossbeam_channel::Sender;
use crate::engine::EngineCommand;

/// Selectable MIDI input backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiBackendType {
    /// Use the `midir` crate for native MIDI input.
    Midir,
    /// Use a `cpal`-shim backend (placeholder for cpal MIDI; may fall back to
    /// midir if no cpal MIDI shim is available on the host).
    CpalShim,
}

impl MidiBackendType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "cpal" | "cpal_shim" => MidiBackendType::CpalShim,
            _ => MidiBackendType::Midir,
        }
    }
}

/// Result of parsing a raw MIDI byte message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiMessage {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    ControlChange { controller: u8, value: u8 },
    Other,
}

/// Parse a standard MIDI message (1–3 bytes).
pub fn parse_midi_message(data: &[u8]) -> MidiMessage {
    if data.is_empty() {
        return MidiMessage::Other;
    }
    let status = data[0] & 0xF0;
    match status {
        0x90 => {
            if data.len() >= 3 {
                let vel = data[2];
                if vel == 0 {
                    MidiMessage::NoteOff { note: data[1] }
                } else {
                    MidiMessage::NoteOn { note: data[1], velocity: vel }
                }
            } else {
                MidiMessage::Other
            }
        }
        0x80 => {
            if data.len() >= 3 {
                MidiMessage::NoteOff { note: data[1] }
            } else {
                MidiMessage::Other
            }
        }
        0xB0 => {
            if data.len() >= 3 {
                MidiMessage::ControlChange { controller: data[1], value: data[2] }
            } else {
                MidiMessage::Other
            }
        }
        _ => MidiMessage::Other,
    }
}

/// Converts a parsed MIDI message into engine commands. Returns `true` if at
/// least one command was produced.
pub fn midi_message_to_commands(msg: MidiMessage, cmd_tx: &Sender<EngineCommand>) -> bool {
    match msg {
        MidiMessage::NoteOn { note, velocity } => {
            let _ = cmd_tx.send(EngineCommand::NoteOn {
                note,
                velocity: velocity as f32 / 127.0,
            });
            true
        }
        MidiMessage::NoteOff { note } => {
            let _ = cmd_tx.send(EngineCommand::NoteOff { note });
            true
        }
        MidiMessage::ControlChange { controller, value } => {
            match controller {
                1 => {
                    // CC1 (mod wheel) -> NN sensitivity
                    let _ = cmd_tx.send(EngineCommand::SetNnSensitivity(value as f32 / 127.0));
                }
                74 => {
                    // CC74 -> prime base
                    let _ = cmd_tx.send(EngineCommand::SetPrimeBase((value as u32).clamp(2, 31)));
                }
                _ => {}
            }
            true
        }
        MidiMessage::Other => false,
    }
}

/// The MIDI manager. Owns the connection to the selected backend and forwards
/// parsed messages to the engine command channel.
pub struct MidiManager {
    pub backend: MidiBackendType,
    _connection: Option<midir::MidiInputConnection<()>>,
}

impl MidiManager {
    pub fn new(backend: MidiBackendType) -> Self {
        Self { backend, _connection: None }
    }

    /// Open the default MIDI input port and forward messages to `cmd_tx`.
    pub fn open_input(&mut self, cmd_tx: Sender<EngineCommand>) -> Result<(), String> {
        match self.backend {
            MidiBackendType::Midir => self.open_midir_input(cmd_tx),
            MidiBackendType::CpalShim => {
                // The cpal MIDI shim is not yet a fully-featured backend on all
                // platforms; attempt midir as a fallback so the device works.
                self.open_midir_input(cmd_tx)
            }
        }
    }

    fn open_midir_input(&mut self, cmd_tx: Sender<EngineCommand>) -> Result<(), String> {
        let midi_in = midir::MidiInput::new("modurust-lite")
            .map_err(|e| format!("failed to create midir input: {e}"))?;

        let ports = midi_in.ports();
        let port = ports
            .first()
            .ok_or_else(|| "no MIDI input ports available".to_string())?;

        let port_name = midi_in.port_name(port).unwrap_or_else(|_| "unknown".to_string());
        println!("[midi] using input port: {port_name}");

        let connection = midi_in
            .connect(
                port,
                "modurust-lite-input",
                move |_stamp, message: &[u8], _| {
                    // Handle running status by accumulating; simplest: parse
                    // each message buffer as a single MIDI message.
                    let msg = parse_midi_message(message);
                    midi_message_to_commands(msg, &cmd_tx);
                },
                (),
            )
            .map_err(|e| format!("failed to connect MIDI input: {e}"))?;

        self._connection = Some(connection);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note_on() {
        let msg = parse_midi_message(&[0x90, 60, 100]);
        assert_eq!(msg, MidiMessage::NoteOn { note: 60, velocity: 100 });
    }

    #[test]
    fn test_parse_note_off() {
        let msg = parse_midi_message(&[0x80, 60, 0]);
        assert_eq!(msg, MidiMessage::NoteOff { note: 60 });
    }

    #[test]
    fn test_parse_velocity_zero_is_note_off() {
        let msg = parse_midi_message(&[0x90, 60, 0]);
        assert_eq!(msg, MidiMessage::NoteOff { note: 60 });
    }

    #[test]
    fn test_parse_cc() {
        let msg = parse_midi_message(&[0xB0, 1, 64]);
        assert_eq!(msg, MidiMessage::ControlChange { controller: 1, value: 64 });
    }

    #[test]
    fn test_backend_from_str() {
        assert_eq!(MidiBackendType::from_str("midir"), MidiBackendType::Midir);
        assert_eq!(MidiBackendType::from_str("cpal"), MidiBackendType::CpalShim);
    }
}
