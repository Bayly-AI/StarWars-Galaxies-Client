// crates/swg-client-ui/src/hud_layout.rs

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// 2D bounding box for vector HUD elements.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    #[must_use]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub fn contains_point(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x <= self.x + self.width && p.y >= self.y && p.y <= self.y + self.height
    }
}

/// Resolution-independent vector UI layout engine scaling from 1080p to 1440p and 4K UHD.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolutionScaler {
    pub screen_width: f32,
    pub screen_height: f32,
    pub scale_factor: f32,
}

impl ResolutionScaler {
    #[must_use]
    pub fn new(width: f32, height: f32) -> Self {
        let width = width.max(800.0);
        let height = height.max(600.0);
        // Base reference is 1920x1080
        let scale_x = width / 1920.0;
        let scale_y = height / 1080.0;
        let scale_factor = scale_x.min(scale_y).max(1.0);

        Self {
            screen_width: width,
            screen_height: height,
            scale_factor,
        }
    }

    /// Scaled vector coordinates for HAM bars (Top-Left).
    #[must_use]
    pub fn ham_bars_rect(&self) -> Rect {
        let s = self.scale_factor;
        Rect::new(24.0 * s, 24.0 * s, 260.0 * s, 72.0 * s)
    }

    /// Scaled vector coordinates for Radar Minimap (Top-Right).
    #[must_use]
    pub fn radar_rect(&self) -> Rect {
        let s = self.scale_factor;
        let size = 180.0 * s;
        Rect::new(self.screen_width - size - 24.0 * s, 24.0 * s, size, size)
    }

    /// Scaled vector coordinates for Selected Target Frame (Top-Center).
    #[must_use]
    pub fn target_frame_rect(&self) -> Rect {
        let s = self.scale_factor;
        let width = 280.0 * s;
        let height = 64.0 * s;
        Rect::new((self.screen_width - width) * 0.5, 24.0 * s, width, height)
    }

    /// Scaled vector coordinates for Macro Button Bar (Bottom-Center).
    #[must_use]
    pub fn button_bar_rect(&self) -> Rect {
        let s = self.scale_factor;
        let width = 640.0 * s;
        let height = 54.0 * s;
        Rect::new(
            (self.screen_width - width) * 0.5,
            self.screen_height - height - 20.0 * s,
            width,
            height,
        )
    }

    /// Scaled vector coordinates for Chat Window (Bottom-Left).
    #[must_use]
    pub fn chat_window_rect(&self) -> Rect {
        let s = self.scale_factor;
        let width = 480.0 * s;
        let height = 220.0 * s;
        Rect::new(
            24.0 * s,
            self.screen_height - height - 24.0 * s,
            width,
            height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_scaling_1080p_1440p_4k() {
        // 1080p (Scale 1.0)
        let scaler_1080 = ResolutionScaler::new(1920.0, 1080.0);
        assert!((scaler_1080.scale_factor - 1.0).abs() < 1e-4);
        assert_eq!(scaler_1080.ham_bars_rect().width, 260.0);

        // 1440p (Scale 1.3333)
        let scaler_1440 = ResolutionScaler::new(2560.0, 1440.0);
        assert!((scaler_1440.scale_factor - (2560.0 / 1920.0)).abs() < 1e-4);
        assert!((scaler_1440.ham_bars_rect().width - (260.0 * 1.333333)).abs() < 1e-2);

        // 4K UHD (Scale 2.0)
        let scaler_4k = ResolutionScaler::new(3840.0, 2160.0);
        assert!((scaler_4k.scale_factor - 2.0).abs() < 1e-4);
        assert_eq!(scaler_4k.ham_bars_rect().width, 520.0);
        assert_eq!(scaler_4k.radar_rect().width, 360.0);
    }
}
