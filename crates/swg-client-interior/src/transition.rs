// crates/swg-client-interior/src/transition.rs

use crate::cell::BuildingInterior;
use glam::Vec3;

/// Manages camera and player coordinate transitions between indoor cells and planetary space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellTransitionManager {
    pub active_cell_id: u64,
}

impl CellTransitionManager {
    #[must_use]
    pub fn new(initial_cell_id: u64) -> Self {
        Self {
            active_cell_id: initial_cell_id,
        }
    }

    /// Evaluates current player world position against building cell bounds and updates active cell.
    pub fn update(&mut self, interior: &BuildingInterior, player_world_pos: Vec3) -> u64 {
        let local_pos = interior.world_to_cell_point(player_world_pos);

        // If currently in a cell, check if still within its bounds
        if self.active_cell_id != 0
            && let Some(current_cell) = interior.get_cell(self.active_cell_id)
            && current_cell.contains_local_point(local_pos)
        {
            return self.active_cell_id;
        }

        // Check if player moved into any cell of this building
        for cell in interior.cells.values() {
            if cell.contains_local_point(local_pos) {
                self.active_cell_id = cell.cell_id;
                return self.active_cell_id;
            }
        }

        // If not inside any cell, player is outdoors in world space
        self.active_cell_id = 0;
        self.active_cell_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellObject;
    use glam::Mat4;

    #[test]
    fn test_cell_transition_in_and_out() {
        let mut interior = BuildingInterior::new(2001, Mat4::IDENTITY);
        interior.add_cell(CellObject::new(
            10,
            2001,
            "cantina_hall",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 5.0, 10.0),
        ));

        let mut manager = CellTransitionManager::new(0); // Starts outdoors

        // Player outside at (-5, 0, 5)
        let cell = manager.update(&interior, Vec3::new(-5.0, 0.0, 5.0));
        assert_eq!(cell, 0);

        // Player walks inside to (5, 2, 5)
        let cell = manager.update(&interior, Vec3::new(5.0, 2.0, 5.0));
        assert_eq!(cell, 10);
        assert_eq!(manager.active_cell_id, 10);

        // Player walks back out to (15, 0, 5)
        let cell = manager.update(&interior, Vec3::new(15.0, 0.0, 5.0));
        assert_eq!(cell, 0);
        assert_eq!(manager.active_cell_id, 0);
    }
}
