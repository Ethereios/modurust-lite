use std::net::UdpSocket;
use std::time::Duration;

/// M4L Instrument Bridge — interactive run-loop with chord progression.
///
/// This bridge simulates a Max for Live instrument device. It binds a local
/// UDP control socket and relays control packets to the Modurust Core Engine
/// (default UDP 9000 where `UdpOscBridge` is listening), and listens for the
/// engine's binary status heartbeat so the instrument UI can display live
/// parameter values.
///
/// It plays a continuous chord progression (C – F – G – Am) with an
/// arpeggiated texture, alternating with sustained chords, exercising the
/// polyphonic SuperInstancePool over the UDP control path.
///
/// Text control protocol (sent to the engine on the control port):
///   `noteon <note> <vel>`   — note on (vel 0-127)
///   `noteoff <note>`        — note off
///   `alloff`                — release all voices
///   `prime <base>`          — set prime base
///   `phase <phase>`         — set prime phase
///   `sens <sens>`           — set NN sensitivity
///   `density <d>`           — set instance density
///   `spread <s>`            — set instance spread
///   `mode <synth|sampler|effect|controller>`
///   `param <name> <value>`  — set a DSP parameter
///   `graph_* ...`           — graph node control (see core/src/comm/mod.rs)
///
/// The engine's status heartbeat is broadcast to the *binary* status ports
/// (status_port+1 .. +3, default 9003-9005). This bridge binds one of them so
/// it can observe live engine state.
///
/// Run: `cargo run -p m4l_instrument`
fn main() {
    let control_port: u16 = std::env::var("MODURUST_UDP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9000);
    let status_port: u16 = std::env::var("MODURUST_STATUS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9003);

    println!("M4L Prime-Complex Instrument Bridge (engine UDP port {control_port}, status port {status_port})");

    let Ok(socket) = UdpSocket::bind("127.0.0.1:0") else {
        eprintln!("failed to create control socket");
        return;
    };
    let _ = socket.connect(("127.0.0.1", control_port));

    let Ok(rx_socket) = UdpSocket::bind(("127.0.0.1", status_port)) else {
        eprintln!(
            "failed to bind status port {status_port} (is another bridge or device already bound?)"
        );
        return;
    };
    let _ = rx_socket.set_read_timeout(Some(Duration::from_millis(200)));
    println!("Listening for engine status heartbeat on port {status_port} (Ctrl+C to stop)...");

    // Chord progression: C major, F major, G major, A minor.
    // Each chord is {root, third, fifth} in MIDI note numbers.
    let progression: [[u8; 3]; 4] = [
        [60, 64, 67], // C major
        [65, 69, 72], // F major
        [67, 71, 74], // G major
        [69, 72, 76], // A minor
    ];

    // Send a few global parameter touches before the sequence starts.
    send(&socket, &format!("prime 5"));
    send(&socket, &format!("sens 0.85"));
    send(&socket, &format!("density 16"));
    send(&socket, &format!("spread 0.5"));
    send(&socket, &format!("param filter_cutoff 0.6"));
    send(&socket, &format!("param reverb_mix 0.4"));

    println!("Playing chord progression C - F - G - Am (Ctrl+C to stop)...");

    let mut cycle: u32 = 0;
    loop {
        let chord = &progression[(cycle % progression.len() as u32) as usize];
        let root_name = match chord[0] {
            60 => "C major",
            65 => "F major",
            67 => "G major",
            _ => "A minor",
        };
        println!("[instrument] playing {root_name} chord {chord:?}");

        // Sustain the chord with a gentle arpeggiation: roll each note on.
        for (i, &note) in chord.iter().enumerate() {
            let vel = 90 + i * 10;
            send(&socket, &format!("noteon {note} {vel}"));
            poll_status(&rx_socket, 3);
            std::thread::sleep(Duration::from_millis(120));
        }

        // Hold the chord while reading a few status heartbeats.
        poll_status(&rx_socket, 4);

        // Release the chord.
        for &note in chord.iter() {
            send(&socket, &format!("noteoff {note}"));
        }

        // Interleave a short prime-base change on even cycles to demonstrate
        // the modulator path from an instrument surface.
        if cycle % 2 == 0 {
            let base = [2u32, 3, 5, 7, 11][(cycle as usize / 2) % 5];
            send(&socket, &format!("prime {base}"));
        }

        cycle += 1;
        poll_status(&rx_socket, 3);
        std::thread::sleep(Duration::from_millis(400));
    }
}

/// Send a text command to the engine control port.
fn send(socket: &UdpSocket, text: &str) {
    let _ = socket.send(text.as_bytes());
    println!("  -> {text}");
}

/// Drain any pending status heartbeats for `max_iters` reads (each with a
/// short timeout), printing decoded engine state.
fn poll_status(rx_socket: &UdpSocket, max_iters: usize) {
    let mut buf = [0u8; 64];
    for _ in 0..max_iters {
        if let Ok((amt, _)) = rx_socket.recv_from(&mut buf) {
            let data = &buf[..amt];
            if data.len() >= 31 && data[0] == 0x4D {
                let mode = match data[2] {
                    1 => "sampler",
                    2 => "effect",
                    3 => "controller",
                    _ => "synth",
                };
                let peak = f32::from_le_bytes([data[3], data[4], data[5], data[6]]);
                let base = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);
                let voices = u32::from_le_bytes([data[27], data[28], data[29], data[30]]);
                println!(
                    "[status] mode={mode} peak={peak:.3} prime_base={base} active_voices={voices}"
                );
            }
        } else {
            break;
        }
    }
}

