// crates/swg-client-app/src/main.rs

use std::env;
use std::time::Duration;
use swg_client_app::{ClientConfig, SwgClientApp};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = ClientConfig::from_args(&args[1..]);

    println!("=============================================================");
    println!(" Star Wars Galaxies: Modern 64-Bit Native Client (Strategy B)");
    println!(" Version: {} (Pre-CU Retail Heritage Edition)", env!("CARGO_PKG_VERSION"));
    println!(" Target Platform: macOS / Metal & Vulkan Architecture");
    println!("=============================================================");
    println!(" Character: {} ({:?})", config.character_name, config.species);
    println!(" Target Planet: {}", config.planet.to_uppercase());
    println!(" Server Endpoint: {}", config.server_addr);
    println!(" Window Resolution: {}x{} ({:?})", config.window_width, config.window_height, config.window_mode);
    println!(" Graphics Backend: {:?}", config.graphics_backend);
    println!("=============================================================");

    let mut app = SwgClientApp::new(config);

    // Mount VFS
    match app.initialize_vfs() {
        Ok(count) => {
            println!("✓ VFS initialized: {} .tre archives mounted ({} virtual assets indexed)", count, app.manifest.total_files);
        }
        Err(e) => {
            eprintln!("⚠ VFS discovery notice: {}", e);
        }
    }

    if app.config.headless {
        println!("Running in headless benchmark mode (100 simulation frames)...");
        for _ in 0..100 {
            app.step_frame();
            std::thread::sleep(Duration::from_millis(1));
        }
        let pos = app.predictor.state().position;
        println!("✓ Headless simulation complete! Final position: ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z);
        return;
    }

    println!("Starting simulation loop (press Ctrl+C to terminate)...");
    for frame in 1..=50 {
        let stats = app.step_frame();
        if frame % 10 == 0 {
            println!(
                "  Frame {:4} | FPS: {:5.1} | Pos: ({:5.1}, {:5.1}, {:5.1}) | UI Verts: {:4} | Theme: {:?}",
                stats.frame_index,
                stats.fps,
                stats.player_position.x,
                stats.terrain_altitude,
                stats.player_position.z,
                stats.ui_vertices_count,
                stats.active_music_theme,
            );
        }
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    println!("✓ Client simulation loop completed successfully!");
}
