// crates/swg-client-ui/src/interaction.rs

use crate::radial::{RadialMenu, RadialOption};
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Target classification for context-sensitive capability generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetKind {
    CreatureCorpse,
    StorageContainer,
    PlayerCharacter,
    ResourceNode,
    MissionTerminal,
    ItemInInventory,
    NpcVendor,
}

pub struct RadialMenuGenerator;

impl RadialMenuGenerator {
    /// Generates authentic Pre-CU radial menu options according to target capabilities.
    #[must_use]
    pub fn build_menu(target_id: u64, kind: TargetKind, click_pos: Vec2) -> RadialMenu {
        let mut menu = RadialMenu::new(target_id, click_pos);

        match kind {
            TargetKind::CreatureCorpse => {
                menu.add_option(RadialOption::new(1, "Loot All", "/loot"));
                menu.add_option(RadialOption::new(2, "Harvest Meat", "/harvest meat"));
                menu.add_option(RadialOption::new(3, "Harvest Hide", "/harvest hide"));
                menu.add_option(RadialOption::new(4, "Harvest Bone", "/harvest bone"));
                menu.add_option(RadialOption::new(5, "Examine", "/examine"));
                menu.default_index = 0; // Default is Loot
            }

            TargetKind::StorageContainer => {
                menu.add_option(RadialOption::new(1, "Open", "/openContainer"));
                menu.add_option(RadialOption::new(2, "Pick Up", "/pickup"));
                menu.add_option(RadialOption::new(3, "Examine", "/examine"));
                menu.default_index = 0; // Default is Open
            }

            TargetKind::PlayerCharacter => {
                menu.add_option(RadialOption::new(1, "Trade", "/trade"));
                menu.add_option(RadialOption::new(2, "Invite", "/invite"));
                menu.add_option(RadialOption::new(3, "Follow", "/follow"));
                menu.add_option(RadialOption::new(4, "Duel", "/duel"));
                menu.add_option(RadialOption::new(5, "Examine", "/examine"));
                menu.default_index = 0;
            }

            TargetKind::ResourceNode => {
                menu.add_option(RadialOption::new(1, "Sample", "/sample"));
                menu.add_option(RadialOption::new(2, "Survey", "/survey"));
                menu.add_option(RadialOption::new(3, "Examine", "/examine"));
                menu.default_index = 0;
            }

            TargetKind::MissionTerminal => {
                menu.add_option(RadialOption::new(1, "Use Terminal", "/useTerminal"));
                menu.add_option(RadialOption::new(2, "Examine", "/examine"));
                menu.default_index = 0;
            }

            TargetKind::ItemInInventory => {
                menu.add_option(RadialOption::new(1, "Use", "/useItem"));
                menu.add_option(RadialOption::new(2, "Destroy", "/destroy"));
                menu.add_option(RadialOption::new(3, "Examine", "/examine"));
                menu.default_index = 0;
            }

            TargetKind::NpcVendor => {
                menu.add_option(RadialOption::new(1, "Browse Goods", "/browseGoods"));
                menu.add_option(RadialOption::new(2, "Converse", "/converse"));
                menu.add_option(RadialOption::new(3, "Examine", "/examine"));
                menu.default_index = 0;
            }
        }

        menu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corpse_and_container_radial_generation() {
        let corpse_menu = RadialMenuGenerator::build_menu(
            801,
            TargetKind::CreatureCorpse,
            Vec2::new(100.0, 100.0),
        );
        assert_eq!(corpse_menu.options.len(), 5);
        assert_eq!(corpse_menu.options[0].command, "/loot");
        assert_eq!(corpse_menu.options[1].command, "/harvest meat");

        let container_menu = RadialMenuGenerator::build_menu(
            802,
            TargetKind::StorageContainer,
            Vec2::new(200.0, 200.0),
        );
        assert_eq!(container_menu.options.len(), 3);
        assert_eq!(container_menu.options[0].command, "/openContainer");
    }
}
