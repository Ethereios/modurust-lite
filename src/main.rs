use bevy::prelude::*;
use hexodsp_daw::{ui::UiState, ui::UiStateRes};
use std::fs;
#[cfg(feature = "webui")]
use std::env;
#[cfg(feature = "webui")]
use std::thread;

// Web interface (only when `webui` feature is enabled)
#[cfg(feature = "webui")]
use hexodsp_daw::web_interface::initialize_web_interface;

const UI_STATE_PATH: &str = "ui_state.json";
use hexodsp_daw::ui::HexoDSPUiPlugin;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Load persisted UI state
    let mut ui_state: UiState = fs::read_to_string(UI_STATE_PATH)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // Sanitize persisted values that can break UI usability
    if !ui_state.font_scale.is_finite() || ui_state.font_scale < 0.5 || ui_state.font_scale > 2.5 {
        ui_state.font_scale = 1.0;
    }

    // Provide loaded UiState to Bevy as a resource

    // Optionally start web interface server only when feature `webui` is enabled
    // and the environment variable HEXODSP_ENABLE_WEB is set.
    #[cfg(feature = "webui")]
    {
        let enable_web = env::var("HEXODSP_ENABLE_WEB")
            .ok()
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        if enable_web {
            // Start web interface server concurrently for preview/demo
            thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime");

                rt.block_on(async move {
                    match initialize_web_interface().await {
                        Ok(server) => {
                            if let Err(e) = server.start().await {
                                eprintln!("Web interface error: {}", e);
                            }
                        }
                        Err(e) => eprintln!("Failed to initialize web interface: {}", e),
                    }
                });
            });
        } else {
            println!("Web interface disabled. Set HEXODSP_ENABLE_WEB=1 to enable.");
        }
    }

    // Build Bevy App
    App::new()
        .insert_resource(UiStateRes(ui_state))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "HexoDSP DAW".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }).disable::<bevy::log::LogPlugin>())
        .add_plugins(HexoDSPUiPlugin)
        .run();

    Ok(())
}
