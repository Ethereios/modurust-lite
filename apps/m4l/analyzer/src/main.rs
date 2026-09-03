use std::net::UdpSocket;
use std::time::Duration;

/// M4L Prime-Complex Analyzer Bridge — interactive run-loop with NN sweep.
///
/// This bridge sends parameter-observation packets to the Core Engine and
/// listens for the engine's binary status heartbeat. It sweeps the complex NN
/// sensitivity from 0.0 → 1.0 and back, so the analyzer device can render
/// live peak / prime / NN state and observe how NN morphing affects the
/// engine's output over time.
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
/// Run: `cargo run -p m4l_analyzer`
fn main() {
    let control_port: u16 = std::env::var("MODURUST_UDP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9000);
    let status_port: u16 = std::env::var("MODURUST_STATUS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9004);

    println!("M4L Prime-Complex Analyzer Bridge (engine UDP port {control_port}, status port {status_port})");

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

    // Warm up the engine with a sustained note so the analyzer has something
    // to measure while the NN sensitivity is swept.
    send(&socket, "noteon 60 100");
    send(&socket, "param filter_cutoff 0.6");
    send(&socket, "param reverb_mix 0.3");

    println!("Sweeping NN sensitivity 0.0 -> 1.0 -> 0.0 (Ctrl+C to stop)...");

    let steps: usize = 20;
    loop {
        // Upward sweep: 0.0 ..= 1.0
        for i in 0..=steps {
            let sens = i as f32 / steps as f32;
            send(&socket, &format!("sens {sens:.3}"));
            poll_status(&rx_socket, 2);
            std::thread::sleep(Duration::from_millis(120));
        }
        // Downward sweep: 1.0 ..= 0.0
        for i in (0..=steps).rev() {
            let sens = i as f32 / steps as f32;
            send(&socket, &format!("sens {sens:.3}"));
            poll_status(&rx_socket, 2);
            std::thread::sleep(Duration::from_millis(120));
        }
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
                let phase = f32::from_le_bytes([data[11], data[12], data[13], data[14]]);
                let sens = f32::from_le_bytes([data[15], data[16], data[17], data[18]]);
                let voices = u32::from_le_bytes([data[27], data[28], data[29], data[30]]);
                println!(
                    "[analyzer] mode={mode} peak={peak:.3} prime_phase={phase:.2} nn_sens={sens:.2} voices={voices}"
                );
            }
        } else {
            break;
        }
    }
}

