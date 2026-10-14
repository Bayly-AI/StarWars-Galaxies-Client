// crates/swg-client-app/src/config.rs

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use swg_client_core::{GraphicsBackend, WindowMode};
use swg_client_customization::{Gender, Species};

/// Graphics hardware API choice for client rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClientGraphicsBackend {
    Auto,
    Vulkan,
    Metal,
    Dx12,
}

impl ClientGraphicsBackend {
    #[must_use]
    pub fn to_core_backend(self) -> GraphicsBackend {
        match self {
            Self::Auto => GraphicsBackend::Auto,
            Self::Vulkan => GraphicsBackend::Vulkan,
            Self::Metal => GraphicsBackend::Metal,
            Self::Dx12 => GraphicsBackend::Dx12,
        }
    }
}

/// Complete launch and runtime configuration for the SWG Client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub server_addr: SocketAddr,
    pub login_account: String,
    pub character_name: String,
    pub species: Species,
    pub gender: Gender,
    pub planet: String,
    pub window_width: u32,
    pub window_height: u32,
    pub window_mode: WindowMode,
    pub graphics_backend: ClientGraphicsBackend,
    pub asset_paths: Vec<PathBuf>,
    pub headless: bool,
    pub fps_limit: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:44453".parse().unwrap(),
            login_account: "player1".to_string(),
            character_name: "Valen Antilles".to_string(),
            species: Species::Human,
            gender: Gender::Male,
            planet: "tatooine".to_string(),
            window_width: 1920,
            window_height: 1080,
            window_mode: WindowMode::Windowed,
            graphics_backend: ClientGraphicsBackend::Auto,
            asset_paths: vec![
                PathBuf::from("downloads/original_client"),
                PathBuf::from("data"),
            ],
            headless: false,
            fps_limit: 144,
        }
    }
}

impl ClientConfig {
    /// Parses CLI argument strings into a configured client instance.
    pub fn from_args<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut config = Self::default();
        let args: Vec<String> = args.into_iter().map(Into::into).collect();

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--server" | "-s" if i + 1 < args.len() => {
                    if let Ok(addr) = args[i + 1].parse() {
                        config.server_addr = addr;
                    }
                    i += 1;
                }
                "--character" | "-c" if i + 1 < args.len() => {
                    config.character_name = args[i + 1].clone();
                    i += 1;
                }
                "--planet" | "-p" if i + 1 < args.len() => {
                    config.planet = args[i + 1].to_lowercase();
                    i += 1;
                }
                "--headless" => {
                    config.headless = true;
                }
                "--width" if i + 1 < args.len() => {
                    if let Ok(w) = args[i + 1].parse() {
                        config.window_width = w;
                    }
                    i += 1;
                }
                "--height" if i + 1 < args.len() => {
                    if let Ok(h) = args[i + 1].parse() {
                        config.window_height = h;
                    }
                    i += 1;
                }
                "--asset-dir" if i + 1 < args.len() => {
                    config.asset_paths.push(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
                _ => {}
            }
            i += 1;
        }

        config
    }
}
