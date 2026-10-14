// crates/swg-client-app/src/app.rs

use crate::config::ClientConfig;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;

use swg_client_audio::{DynamicMusicDirector, ThemeType};
use swg_client_customization::{CharacterAppearance, PaletteLibrary};
use swg_client_render::TerrainMaterialPalette;
use swg_client_sync::{InputCommand, MovementPredictor, PlayerPhysicsConfig, PlayerPhysicsState};
use swg_client_terrain::TerrainGenerator;
use swg_client_ui::{ButtonBar, CharacterHam, PreCuUiRenderer, RadarMinimap, ResolutionScaler, UiMeshBuffer};
use swg_client_vfs::{AssetExtractor, AssetManifest, VfsError, VfsManager};
use swg_client_world::SpatialSceneGraph;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("VFS initialization failed: {0}")]
    Vfs(#[from] VfsError),
    #[error("Renderer error: {0}")]
    Render(String),
}

/// Real-time frame statistics for telemetry and HUD diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FrameStats {
    pub frame_index: u64,
    pub delta_time_seconds: f32,
    pub fps: f32,
    pub player_position: Vec3,
    pub terrain_altitude: f32,
    pub ui_vertices_count: usize,
    pub active_music_theme: ThemeType,
}

/// The integrated modern 64-bit Star Wars Galaxies Client Application.
pub struct SwgClientApp {
    pub config: ClientConfig,
    pub vfs: VfsManager,
    pub manifest: AssetManifest,
    pub terrain: TerrainGenerator,
    pub terrain_palette: TerrainMaterialPalette,
    pub player: CharacterAppearance,
    pub palettes: PaletteLibrary,
    pub ham: CharacterHam,
    pub radar: RadarMinimap,
    pub button_bar: ButtonBar,
    pub music: DynamicMusicDirector,
    pub predictor: MovementPredictor,
    pub scene: SpatialSceneGraph,
    pub ui_scaler: ResolutionScaler,
    pub ui_mesh: UiMeshBuffer,
    pub frame_counter: u64,
    pub last_frame_time: Instant,
    pub is_running: bool,
}

impl SwgClientApp {
    /// Initializes client application state from configuration.
    pub fn new(config: ClientConfig) -> Self {
        let terrain = if config.planet == "naboo" {
            TerrainGenerator::naboo()
        } else {
            TerrainGenerator::tatooine()
        };

        let player = CharacterAppearance::new(config.species, config.gender);
        let palettes = PaletteLibrary::new();
        let ham = CharacterHam::new(1200, 1000, 900);
        let radar = RadarMinimap::new(Vec3::new(0.0, 10.0, 0.0), 0.0, 128.0);
        let button_bar = ButtonBar::new();
        let music = DynamicMusicDirector::new();
        let predictor = MovementPredictor::new(
            PlayerPhysicsState::new(Vec3::new(0.0, 10.0, 0.0)),
            PlayerPhysicsConfig::default(),
        );
        let scene = SpatialSceneGraph::new(64.0);
        let ui_scaler = ResolutionScaler::new(config.window_width as f32, config.window_height as f32);
        let ui_mesh = UiMeshBuffer::new();

        Self {
            config,
            vfs: VfsManager::new(),
            manifest: AssetManifest::new(),
            terrain,
            terrain_palette: TerrainMaterialPalette::tatooine(),
            player,
            palettes,
            ham,
            radar,
            button_bar,
            music,
            predictor,
            scene,
            ui_scaler,
            ui_mesh,
            frame_counter: 0,
            last_frame_time: Instant::now(),
            is_running: true,
        }
    }

    /// Discovers and mounts all assets from configured search directories.
    pub fn initialize_vfs(&mut self) -> Result<usize, AppError> {
        let mut total_mounted = 0;
        for path in &self.config.asset_paths {
            if path.exists() {
                if let Ok(manifest) = AssetExtractor::ingest_directory(path, &mut self.vfs) {
                    total_mounted += manifest.mounted_archives.len();
                    self.manifest.total_files += manifest.total_files;
                    for (cat, count) in manifest.category_counts {
                        *self.manifest.category_counts.entry(cat).or_insert(0) += count;
                    }
                }
            }
        }
        Ok(total_mounted)
    }

    /// Advances simulation by time step `dt`.
    pub fn tick(&mut self, dt: f32) {
        // 1. Advance predicted movement
        let input = InputCommand {
            delta_time: dt,
            forward: 1.0,
            strafe: 0.0,
            jump: false,
            sprint: false,
            yaw: 0.0,
            pitch: 0.0,
            timestamp: self.frame_counter as f64 * 0.016,
        };
        let (_seq, predicted_state) = self.predictor.push_input(input);

        // Clamp height to terrain elevation
        let height = self.terrain.get_height(predicted_state.position.x, predicted_state.position.z);
        let adjusted_pos = Vec3::new(predicted_state.position.x, height, predicted_state.position.z);
        self.radar.center_pos = adjusted_pos;
        self.radar.heading = predicted_state.yaw;

        // 2. Advance dynamic soundtrack
        self.music.tick(dt);

        // 3. Assemble Pre-CU HUD vector mesh
        self.ui_mesh.clear();
        let ham_rect = self.ui_scaler.ham_bars_rect();
        PreCuUiRenderer::render_ham_bars(&self.ham, ham_rect, &mut self.ui_mesh);

        let radar_rect = self.ui_scaler.radar_rect();
        PreCuUiRenderer::render_radar(&self.radar, radar_rect, &mut self.ui_mesh);

        // Macro button bar
        let btn_rect = self.ui_scaler.button_bar_rect();
        self.ui_mesh.add_beveled_panel(
            btn_rect,
            swg_client_ui::colors::CARBONITE_BG,
            swg_client_ui::colors::BORDER_CYAN_DIM,
            1.0,
        );

        self.frame_counter += 1;
    }

    /// Executes one single tick and returns frame statistics.
    pub fn step_frame(&mut self) -> FrameStats {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32().max(0.001);
        self.last_frame_time = now;

        self.tick(dt);

        let pos = self.predictor.state().position;
        let alt = self.terrain.get_height(pos.x, pos.z);

        FrameStats {
            frame_index: self.frame_counter,
            delta_time_seconds: dt,
            fps: 1.0 / dt,
            player_position: pos,
            terrain_altitude: alt,
            ui_vertices_count: self.ui_mesh.vertices.len(),
            active_music_theme: self.music.current_theme,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_app_initialization_and_frame_loop() {
        let mut config = ClientConfig::default();
        config.headless = true;
        config.character_name = "Corran Horn".to_string();

        let mut app = SwgClientApp::new(config);
        assert_eq!(app.frame_counter, 0);
        assert_eq!(app.player.species, swg_client_customization::Species::Human);

        // Run 10 simulation frames
        for _ in 0..10 {
            let stats = app.step_frame();
            assert!(stats.frame_index > 0);
            assert!(stats.ui_vertices_count > 0);
            assert_eq!(stats.active_music_theme, ThemeType::PlanetaryAmbient);
        }

        assert_eq!(app.frame_counter, 10);
    }

    #[test]
    fn test_client_app_combat_soundtrack_and_vitals() {
        let mut app = SwgClientApp::new(ClientConfig::default());
        // Enter combat
        app.music.enter_combat(0.5);
        app.ham.health.apply_damage(150);
        app.ham.health.apply_wound(50);

        let stats = app.step_frame();
        assert_eq!(app.ham.health.current, 1050);
        assert_eq!(app.ham.health.wound, 50);
        assert!(stats.ui_vertices_count > 0);
    }
}
