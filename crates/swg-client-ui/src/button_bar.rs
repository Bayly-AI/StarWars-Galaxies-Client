// crates/swg-client-ui/src/button_bar.rs

use serde::{Deserialize, Serialize};

/// A single hotkey slot on the macro button bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroSlot {
    pub slot_index: usize,
    pub label: String,
    pub command: String,
    pub cooldown_remaining: f32,
    pub cooldown_total: f32,
}

impl MacroSlot {
    #[must_use]
    pub fn new(
        slot_index: usize,
        label: impl Into<String>,
        command: impl Into<String>,
        cooldown_total: f32,
    ) -> Self {
        Self {
            slot_index,
            label: label.into(),
            command: command.into(),
            cooldown_remaining: 0.0,
            cooldown_total: cooldown_total.max(0.0),
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.cooldown_remaining <= 0.001
    }

    /// Triggers execution of the command if ready, starting cooldown clock.
    pub fn trigger(&mut self) -> Option<String> {
        if self.is_ready() {
            self.cooldown_remaining = self.cooldown_total;
            Some(self.command.clone())
        } else {
            None
        }
    }

    /// Advances cooldown timer.
    pub fn update(&mut self, dt: f32) {
        if self.cooldown_remaining > 0.0 {
            self.cooldown_remaining = (self.cooldown_remaining - dt).max(0.0);
        }
    }
}

/// 12-slot macro action button bar with multi-page switching (F1-F12 / 1-0).
pub struct ButtonBar {
    pub current_page: usize,
    slots: [Option<MacroSlot>; 12],
}

impl Default for ButtonBar {
    fn default() -> Self {
        Self {
            current_page: 1,
            slots: Default::default(),
        }
    }
}

impl ButtonBar {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Assigns a macro command to a specific slot (0..11).
    pub fn assign(
        &mut self,
        slot_index: usize,
        label: impl Into<String>,
        command: impl Into<String>,
        cooldown: f32,
    ) {
        if slot_index < 12 {
            self.slots[slot_index] = Some(MacroSlot::new(slot_index, label, command, cooldown));
        }
    }

    /// Clears a slot.
    pub fn clear_slot(&mut self, slot_index: usize) {
        if slot_index < 12 {
            self.slots[slot_index] = None;
        }
    }

    /// Gets slot reference.
    #[must_use]
    pub fn get_slot(&self, slot_index: usize) -> Option<&MacroSlot> {
        if slot_index < 12 {
            self.slots[slot_index].as_ref()
        } else {
            None
        }
    }

    /// Triggers slot execution via hotkey press (0..11).
    pub fn press_slot(&mut self, slot_index: usize) -> Option<String> {
        if slot_index < 12
            && let Some(slot) = &mut self.slots[slot_index]
        {
            return slot.trigger();
        }
        None
    }

    /// Updates cooldowns across all slots on the active bar.
    pub fn update(&mut self, dt: f32) {
        for slot in self.slots.iter_mut().flatten() {
            slot.update(dt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_bar_trigger_and_cooldown() {
        let mut bar = ButtonBar::new();
        bar.assign(0, "Burst", "/burstRun", 30.0);
        bar.assign(1, "Attack", "/attack", 1.5);

        // Slot 0 triggers immediately
        let cmd = bar.press_slot(0);
        assert_eq!(cmd.as_deref(), Some("/burstRun"));

        // Slot 0 is on cooldown
        let cmd2 = bar.press_slot(0);
        assert!(cmd2.is_none());

        // Advance 15s: still on cooldown
        bar.update(15.0);
        assert!(!bar.get_slot(0).unwrap().is_ready());

        // Advance another 15s: ready again
        bar.update(15.0);
        assert!(bar.get_slot(0).unwrap().is_ready());
        assert_eq!(bar.press_slot(0).as_deref(), Some("/burstRun"));
    }
}
