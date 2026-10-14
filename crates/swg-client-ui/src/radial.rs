// crates/swg-client-ui/src/radial.rs

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// A single selectable slice in a context radial menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadialOption {
    pub id: u32,
    pub label: String,
    pub command: String,
    pub enabled: bool,
}

impl RadialOption {
    #[must_use]
    pub fn new(id: u32, label: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            command: command.into(),
            enabled: true,
        }
    }
}

/// The authentic Pre-CU context-sensitive radial menu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadialMenu {
    pub target_id: u64,
    pub center_px: Vec2,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub options: Vec<RadialOption>,
    pub default_index: usize,
}

impl RadialMenu {
    #[must_use]
    pub fn new(target_id: u64, center_px: Vec2) -> Self {
        Self {
            target_id,
            center_px,
            inner_radius: 30.0,
            outer_radius: 110.0,
            options: Vec::new(),
            default_index: 0,
        }
    }

    pub fn add_option(&mut self, option: RadialOption) {
        self.options.push(option);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.options.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    /// Selects an option using a 1-based number key (1..=options.len()).
    #[must_use]
    pub fn select_by_hotkey(&self, key_number: usize) -> Option<&RadialOption> {
        if key_number >= 1 && key_number <= self.options.len() {
            let opt = &self.options[key_number - 1];
            if opt.enabled {
                return Some(opt);
            }
        }
        None
    }

    /// Evaluates mouse cursor position against radial slices.
    /// If hovering inside inner radius, selects default action.
    /// Otherwise finds slice by angular quadrant.
    #[must_use]
    pub fn select_by_mouse(&self, mouse_px: Vec2) -> Option<&RadialOption> {
        if self.options.is_empty() {
            return None;
        }

        let delta = mouse_px - self.center_px;
        let dist = delta.length();

        // 1. Hovering directly over center core: trigger default option
        if dist <= self.inner_radius {
            if self.default_index < self.options.len() {
                let opt = &self.options[self.default_index];
                if opt.enabled {
                    return Some(opt);
                }
            }
            return None;
        }

        // 2. Beyond outer menu threshold: no selection
        if dist > self.outer_radius * 1.5 {
            return None;
        }

        // 3. Angular slice selection: 0 is Up (-Y in screen space)
        let angle = delta.x.atan2(-delta.y).rem_euclid(2.0 * PI);
        let n = self.options.len();
        let slice_span = (2.0 * PI) / (n as f32);

        // Offset by half-slice so option 0 is centered directly at top
        let shifted_angle = (angle + slice_span * 0.5).rem_euclid(2.0 * PI);
        let index = ((shifted_angle / slice_span).floor() as usize) % n;

        if index < self.options.len() {
            let opt = &self.options[index];
            if opt.enabled {
                return Some(opt);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radial_slice_selection_by_angle() {
        let mut menu = RadialMenu::new(4001, Vec2::new(500.0, 500.0));
        menu.add_option(RadialOption::new(1, "Examine", "/examine")); // 0: Top
        menu.add_option(RadialOption::new(2, "Trade", "/trade")); // 1: Right
        menu.add_option(RadialOption::new(3, "Follow", "/follow")); // 2: Bottom
        menu.add_option(RadialOption::new(4, "Invite", "/invite")); // 3: Left

        // Center hover: default option (0 -> Examine)
        let center_sel = menu.select_by_mouse(Vec2::new(500.0, 500.0));
        assert_eq!(center_sel.map(|o| o.label.as_str()), Some("Examine"));

        // Mouse moved Up to (500, 440) -> Top slice (Examine)
        let top_sel = menu.select_by_mouse(Vec2::new(500.0, 440.0));
        assert_eq!(top_sel.map(|o| o.label.as_str()), Some("Examine"));

        // Mouse moved Right to (560, 500) -> Right slice (Trade)
        let right_sel = menu.select_by_mouse(Vec2::new(560.0, 500.0));
        assert_eq!(right_sel.map(|o| o.label.as_str()), Some("Trade"));

        // Hotkey selection: key 4 -> Invite
        let hotkey_sel = menu.select_by_hotkey(4);
        assert_eq!(hotkey_sel.map(|o| o.label.as_str()), Some("Invite"));
    }
}
