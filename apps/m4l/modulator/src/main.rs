use std::net::UdpSocket;
use std::time::Duration;

/// M4L Prime-Complex Modulator Bridge — interactive run-loop with prime sweep.
///
/// This bridge sweeps the prime sequence base (2, 3, 5, 7, 11, 13, 17) and
/// cycles instance density/spread so the Core Engine's prime-based pattern
/// generator and voice pool are continuously re-tuned. It listens for the
/// engine's binary status heartbeat to verify each modulation took effect.
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
///
/// Run: `cargo run -p m4l_modulator`
fn main() {
    let control_port: u16 = std::env::var("MODURUST_UDP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9000);
    let status_port: u16 = std::env::var("MODURUST_STATUS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9005);

    println!("M4L Prime-Complex Modulator Bridge (engine UDP port {control_port}, status port {status_port})");

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

    // Prime bases to sweep through.
    let primes: [u32; 7] = [2, 3, 5, 7, 11, 13, 17];
    let densities: [u32; 4] = [8, 16, 32, 48];
    let spreads: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

    // Warm up with a held chord so modulation is audible.
    send(&socket, "noteon 60 90");
    send(&socket, "noteon 64 80");
    send(&socket, "noteon 67 70");

    println!("Sweeping prime base / density / spread (Ctrl+C to stop)...");

    let mut idx = 0usize;
    loop {
        let base = primes[idx % primes.len()];
        let density = densities[(idx / 2) % densities.len()];
        let spread = spreads[(idx / 4) % spreads.len()];

        send(&socket, &format!("prime {base}"));
        send(&socket, &format!("density {density}"));
        send(&socket, &format!("spread {spread:.2}"));

        println!(
            "[modulator] applied prime_base={base} density={density} spread={spread:.2}"
        );
        poll_status(&rx_socket, 3);
        std::thread::sleep(Duration::from_millis(350));

        idx += 1;
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
                let base = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);
                let density = u32::from_le_bytes([data[19], data[20], data[21], data[22]]);
                let spread = f32::from_le_bytes([data[23], data[24], data[25], data[26]]);
                let voices = u32::from_le_bytes([data[27], data[28], data[29], data[30]]);
                println!(
                    "[modulator] prime_base={base} density={density} spread={spread:.2} voices={voices}"
                );
            }
        } else {
            break;
        }
    }
}

