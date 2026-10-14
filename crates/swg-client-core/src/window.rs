// crates/swg-client-core/src/window.rs

use serde::{Deserialize, Serialize};

/// Presentation mode configuring vertical synchronization and frame rate limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PresentModeSetting {
    /// V-Sync enabled; frame rate is capped to monitor refresh rate (60/120/144Hz).
    #[default]
    Fifo,
    /// Uncapped, non-tearing mailbox mode. Ideal for low-latency 144Hz+ competitive rendering.
    Mailbox,
    /// Immediate presentation with potential tearing; lowest possible latency.
    Immediate,
    /// Adaptive V-Sync presentation mode.
    AutoVsync,
}

impl From<PresentModeSetting> for wgpu::PresentMode {
    fn from(mode: PresentModeSetting) -> Self {
        match mode {
            PresentModeSetting::Fifo => wgpu::PresentMode::Fifo,
            PresentModeSetting::Mailbox => wgpu::PresentMode::Mailbox,
            PresentModeSetting::Immediate => wgpu::PresentMode::Immediate,
            PresentModeSetting::AutoVsync => wgpu::PresentMode::AutoVsync,
        }
    }
}

/// Window display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WindowMode {
    #[default]
    Windowed,
    BorderlessFullscreen,
    ExclusiveFullscreen,
}

/// Window configuration options including High-DPI and refresh rate settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub mode: WindowMode,
    pub present_mode: PresentModeSetting,
    pub target_fps: f64,
    pub resizable: bool,
    pub high_dpi_enabled: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Star Wars Galaxies — Modern Client".to_string(),
            width: 1920,
            height: 1080,
            mode: WindowMode::Windowed,
            present_mode: PresentModeSetting::Mailbox,
            target_fps: 144.0,
            resizable: true,
            high_dpi_enabled: true,
        }
    }
}

/// High-DPI metrics calculator for crisp rendering without blur on Retina/4K displays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayMetrics {
    pub logical_width: f64,
    pub logical_height: f64,
    pub scale_factor: f64,
    pub physical_width: u32,
    pub physical_height: u32,
}

impl DisplayMetrics {
    /// Creates display metrics from logical dimensions and a platform scale factor.
    #[must_use]
    pub fn new(logical_width: f64, logical_height: f64, scale_factor: f64) -> Self {
        let valid_scale = if scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        let physical_width = (logical_width * valid_scale).round().max(1.0) as u32;
        let physical_height = (logical_height * valid_scale).round().max(1.0) as u32;

        Self {
            logical_width,
            logical_height,
            scale_factor: valid_scale,
            physical_width,
            physical_height,
        }
    }

    /// Converts physical screen coordinates to logical UI coordinates.
    #[must_use]
    pub fn physical_to_logical(&self, physical_x: f64, physical_y: f64) -> (f64, f64) {
        (
            physical_x / self.scale_factor,
            physical_y / self.scale_factor,
        )
    }

    /// Converts logical coordinates to physical pixel coordinates.
    #[must_use]
    pub fn logical_to_physical(&self, logical_x: f64, logical_y: f64) -> (f64, f64) {
        (logical_x * self.scale_factor, logical_y * self.scale_factor)
    }

    /// Aspect ratio (width / height) based on physical pixel dimensions.
    #[must_use]
    pub fn aspect_ratio(&self) -> f32 {
        if self.physical_height == 0 {
            1.0
        } else {
            self.physical_width as f32 / self.physical_height as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_metrics_high_dpi() {
        let metrics = DisplayMetrics::new(1920.0, 1080.0, 2.0);
        assert_eq!(metrics.physical_width, 3840);
        assert_eq!(metrics.physical_height, 2160);
        assert!((metrics.aspect_ratio() - (16.0 / 9.0)).abs() < 1e-4);

        let (lx, ly) = metrics.physical_to_logical(3840.0, 2160.0);
        assert_eq!(lx, 1920.0);
        assert_eq!(ly, 1080.0);

        let (px, py) = metrics.logical_to_physical(960.0, 540.0);
        assert_eq!(px, 1920.0);
        assert_eq!(py, 1080.0);
    }

    #[test]
    fn test_present_mode_conversion() {
        assert_eq!(
            wgpu::PresentMode::from(PresentModeSetting::Mailbox),
            wgpu::PresentMode::Mailbox
        );
        assert_eq!(
            wgpu::PresentMode::from(PresentModeSetting::Immediate),
            wgpu::PresentMode::Immediate
        );
        assert_eq!(
            wgpu::PresentMode::from(PresentModeSetting::Fifo),
            wgpu::PresentMode::Fifo
        );
    }
}
