// crates/swg-client-ui/src/theme.rs

use crate::ham::CharacterHam;
use crate::hud_layout::Rect;
use crate::radar::{BlipType, RadarMinimap};
use crate::radial::RadialMenu;
use serde::{Deserialize, Serialize};

/// Authentic 2003 Pre-CU interface colors.
pub mod colors {
    pub const CARBONITE_BG: [u8; 4] = [10, 18, 30, 230];
    pub const CARBONITE_BG_LIGHT: [u8; 4] = [20, 32, 50, 240];
    pub const BORDER_CYAN: [u8; 4] = [0, 212, 255, 255];
    pub const BORDER_CYAN_DIM: [u8; 4] = [0, 140, 180, 200];
    pub const ACTIVE_AMBER: [u8; 4] = [255, 184, 0, 255];
    pub const TEXT_PRIMARY: [u8; 4] = [240, 245, 255, 255];
    pub const TEXT_MUTED: [u8; 4] = [160, 180, 205, 255];

    // HAM vitals
    pub const HEALTH_BAR: [u8; 4] = [255, 59, 48, 255];
    pub const HEALTH_WOUND: [u8; 4] = [60, 15, 15, 255];
    pub const ACTION_BAR: [u8; 4] = [52, 199, 89, 255];
    pub const ACTION_WOUND: [u8; 4] = [15, 50, 20, 255];
    pub const MIND_BAR: [u8; 4] = [0, 122, 255, 255];
    pub const MIND_WOUND: [u8; 4] = [10, 25, 60, 255];

    // Radar blip colors
    pub const BLIP_PLAYER: [u8; 4] = [255, 255, 255, 255];
    pub const BLIP_FRIEND: [u8; 4] = [52, 199, 89, 255];
    pub const BLIP_NEUTRAL: [u8; 4] = [255, 204, 0, 255];
    pub const BLIP_ENEMY: [u8; 4] = [255, 59, 48, 255];
    pub const BLIP_WAYPOINT: [u8; 4] = [0, 212, 255, 255];
    pub const BLIP_GROUP: [u8; 4] = [175, 82, 222, 255];
}

/// A 2D textured or colored vertex for vector UI rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

/// Generated 2D mesh batch for UI frame drawing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UiMeshBuffer {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
}

impl UiMeshBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    /// Appends a solid-color quad.
    pub fn add_rect(&mut self, rect: Rect, color: [u8; 4]) {
        let base_idx = self.vertices.len() as u32;
        let x0 = rect.x;
        let y0 = rect.y;
        let x1 = rect.x + rect.width;
        let y1 = rect.y + rect.height;

        self.vertices.push(UiVertex {
            position: [x0, y0],
            uv: [0.0, 0.0],
            color,
        });
        self.vertices.push(UiVertex {
            position: [x1, y0],
            uv: [1.0, 0.0],
            color,
        });
        self.vertices.push(UiVertex {
            position: [x1, y1],
            uv: [1.0, 1.0],
            color,
        });
        self.vertices.push(UiVertex {
            position: [x0, y1],
            uv: [0.0, 1.0],
            color,
        });

        self.indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx,
            base_idx + 2,
            base_idx + 3,
        ]);
    }

    /// Appends a framed beveled panel with authentic Pre-CU corner borders.
    pub fn add_beveled_panel(
        &mut self,
        rect: Rect,
        bg_color: [u8; 4],
        border_color: [u8; 4],
        border_thickness: f32,
    ) {
        // Inner background
        self.add_rect(rect, bg_color);

        let t = border_thickness;
        // Top border
        self.add_rect(Rect::new(rect.x, rect.y, rect.width, t), border_color);
        // Bottom border
        self.add_rect(
            Rect::new(rect.x, rect.y + rect.height - t, rect.width, t),
            border_color,
        );
        // Left border
        self.add_rect(Rect::new(rect.x, rect.y, t, rect.height), border_color);
        // Right border
        self.add_rect(
            Rect::new(rect.x + rect.width - t, rect.y, t, rect.height),
            border_color,
        );

        // Pre-CU Corner accents (4 little glowing notches)
        let notch_size = 4.0;
        self.add_rect(
            Rect::new(rect.x - 1.0, rect.y - 1.0, notch_size, notch_size),
            colors::ACTIVE_AMBER,
        );
        self.add_rect(
            Rect::new(
                rect.x + rect.width - notch_size + 1.0,
                rect.y - 1.0,
                notch_size,
                notch_size,
            ),
            colors::ACTIVE_AMBER,
        );
        self.add_rect(
            Rect::new(
                rect.x - 1.0,
                rect.y + rect.height - notch_size + 1.0,
                notch_size,
                notch_size,
            ),
            colors::ACTIVE_AMBER,
        );
        self.add_rect(
            Rect::new(
                rect.x + rect.width - notch_size + 1.0,
                rect.y + rect.height - notch_size + 1.0,
                notch_size,
                notch_size,
            ),
            colors::ACTIVE_AMBER,
        );
    }
}

/// Renderer responsible for assembling the complete Pre-CU vector HUD interface.
pub struct PreCuUiRenderer;

impl PreCuUiRenderer {
    /// Renders HAM (Health, Action, Mind) bars with wound notches and capacity background.
    pub fn render_ham_bars(ham: &CharacterHam, rect: Rect, mesh: &mut UiMeshBuffer) {
        // Panel background
        mesh.add_beveled_panel(rect, colors::CARBONITE_BG, colors::BORDER_CYAN, 1.5);

        let bar_margin = 6.0;
        let bar_spacing = 4.0;
        let available_w = rect.width - bar_margin * 2.0;
        let available_h = rect.height - bar_margin * 2.0;
        let bar_h = (available_h - bar_spacing * 2.0) / 3.0;

        let pools = [
            (&ham.health, colors::HEALTH_BAR, colors::HEALTH_WOUND),
            (&ham.action, colors::ACTION_BAR, colors::ACTION_WOUND),
            (&ham.mind, colors::MIND_BAR, colors::MIND_WOUND),
        ];

        for (i, (pool, bar_col, wound_col)) in pools.iter().enumerate() {
            let by = rect.y + bar_margin + (bar_h + bar_spacing) * i as f32;
            let bar_rect = Rect::new(rect.x + bar_margin, by, available_w, bar_h);

            // Pool background (unwounded capacity)
            mesh.add_rect(bar_rect, [25, 30, 40, 255]);

            // Wound deficit segment (right side)
            let max_val = pool.max.max(1) as f32;
            let wound_w = (pool.wound as f32 / max_val) * available_w;
            if wound_w > 0.0 {
                let wx = bar_rect.x + bar_rect.width - wound_w;
                mesh.add_rect(Rect::new(wx, by, wound_w, bar_h), *wound_col);
            }

            // Current active vitals bar (left side)
            let effective_max = pool.effective_max().max(1) as f32;
            let current_ratio = (pool.current as f32 / effective_max).clamp(0.0, 1.0);
            let active_w = (available_w - wound_w) * current_ratio;
            if active_w > 0.0 {
                mesh.add_rect(Rect::new(bar_rect.x, by, active_w, bar_h), *bar_col);
            }
        }
    }

    /// Renders the circular planetary radar minimap with cardinal indicators and entity blips.
    pub fn render_radar(radar: &RadarMinimap, rect: Rect, mesh: &mut UiMeshBuffer) {
        // Outer bezel
        mesh.add_beveled_panel(rect, colors::CARBONITE_BG, colors::BORDER_CYAN, 1.5);

        let center_x = rect.x + rect.width * 0.5;
        let center_y = rect.y + rect.height * 0.5;
        let radius = (rect.width * 0.5) - 8.0;

        // Center player indicator (white diamond)
        mesh.add_rect(
            Rect::new(center_x - 3.0, center_y - 3.0, 6.0, 6.0),
            colors::BLIP_PLAYER,
        );

        // Blips
        for blip in &radar.blips {
            let col = match blip.blip_type {
                BlipType::FriendlyPlayer => colors::BLIP_FRIEND,
                BlipType::NeutralNpc => colors::BLIP_NEUTRAL,
                BlipType::HostileEnemy => colors::BLIP_ENEMY,
                BlipType::Creature => colors::BLIP_NEUTRAL,
                BlipType::Waypoint => colors::BLIP_WAYPOINT,
                BlipType::Corpse => colors::BLIP_GROUP,
            };

            let (offset, _clamped) = radar.compute_blip_screenspace(blip.world_pos, radius);
            mesh.add_rect(
                Rect::new(center_x + offset.x - 2.5, center_y + offset.y - 2.5, 5.0, 5.0),
                col,
            );
        }
    }

    /// Renders contextual 8-slice radial menu.
    pub fn render_radial_menu(
        radial: &RadialMenu,
        selected_index: Option<usize>,
        mesh: &mut UiMeshBuffer,
    ) {
        let center = radial.center_px;
        let hub_size = radial.inner_radius * 1.2;
        mesh.add_beveled_panel(
            Rect::new(center.x - hub_size * 0.5, center.y - hub_size * 0.5, hub_size, hub_size),
            colors::CARBONITE_BG_LIGHT,
            colors::ACTIVE_AMBER,
            1.5,
        );

        // Radial petal slices
        let count = radial.options.len();
        if count == 0 {
            return;
        }

        let slice_angle = std::f32::consts::TAU / count as f32;
        let radius = (radial.inner_radius + radial.outer_radius) * 0.5;

        for (i, _opt) in radial.options.iter().enumerate() {
            let angle = i as f32 * slice_angle - std::f32::consts::FRAC_PI_2;
            let px = center.x + angle.cos() * radius;
            let py = center.y + angle.sin() * radius;

            let is_selected = selected_index == Some(i);
            let bg_col = if is_selected {
                colors::ACTIVE_AMBER
            } else {
                colors::CARBONITE_BG
            };
            let border_col = if is_selected {
                colors::TEXT_PRIMARY
            } else {
                colors::BORDER_CYAN
            };

            let petal_w = 70.0;
            let petal_h = 24.0;
            mesh.add_beveled_panel(
                Rect::new(px - petal_w * 0.5, py - petal_h * 0.5, petal_w, petal_h),
                bg_col,
                border_col,
                1.0,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radar::RadarBlip;
    use crate::radial::RadialOption;
    use glam::{Vec2, Vec3};

    #[test]
    fn test_ui_mesh_buffer_quad_generation() {
        let mut mesh = UiMeshBuffer::new();
        mesh.add_rect(Rect::new(10.0, 10.0, 100.0, 50.0), colors::BORDER_CYAN);

        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        assert_eq!(mesh.vertices[0].color, colors::BORDER_CYAN);
    }

    #[test]
    fn test_ham_bars_drawing_vertices() {
        let mut mesh = UiMeshBuffer::new();
        let mut ham = CharacterHam::new(1000, 1000, 1000);
        ham.health.apply_damage(200);
        ham.health.apply_wound(100);

        let rect = Rect::new(24.0, 24.0, 260.0, 72.0);
        PreCuUiRenderer::render_ham_bars(&ham, rect, &mut mesh);

        assert!(mesh.vertices.len() > 10, "HAM bars must generate multiple quads");
        assert!(mesh.indices.len() > 15);
    }

    #[test]
    fn test_radar_and_radial_drawing() {
        let mut mesh = UiMeshBuffer::new();
        let mut radar = RadarMinimap::new(Vec3::ZERO, 0.0, 128.0);
        radar.add_blip(RadarBlip {
            id: 101,
            world_pos: Vec3::new(50.0, 0.0, 50.0),
            blip_type: BlipType::HostileEnemy,
            label: "Krayt Dragon".to_string(),
        });

        let radar_rect = Rect::new(800.0, 24.0, 180.0, 180.0);
        PreCuUiRenderer::render_radar(&radar, radar_rect, &mut mesh);
        assert!(mesh.vertices.len() >= 8);

        // Radial menu
        let mut radial = RadialMenu::new(42, Vec2::new(400.0, 300.0));
        radial.add_option(RadialOption::new(1, "Examine", "/examine"));
        radial.add_option(RadialOption::new(2, "Open", "/open"));

        PreCuUiRenderer::render_radial_menu(&radial, Some(0), &mut mesh);
        assert!(mesh.vertices.len() > 20);
    }
}
