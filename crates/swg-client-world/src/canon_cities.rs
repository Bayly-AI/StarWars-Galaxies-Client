// crates/swg-client-world/src/canon_cities.rs

use crate::ws::{WorldSnapshot, WorldSnapshotNode};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlanetHub {
    TatooineMosEisley,
    NabooTheed,
    CorelliaCoronet,
}

pub struct CanonCityGenerator;

impl CanonCityGenerator {
    /// Generates retail canon static structures for a specific planetary city hub.
    #[must_use]
    pub fn generate_hub_snapshot(hub: PlanetHub) -> WorldSnapshot {
        let mut snapshot = WorldSnapshot::new();

        match hub {
            PlanetHub::TatooineMosEisley => {
                // Chalmun's Cantina
                snapshot.add_node(WorldSnapshotNode::new(
                    100_001,
                    "object/building/tatooine/shared_cantina_tatooine.iff",
                    0,
                    Vec3::new(3528.0, 5.0, -4804.0),
                    Quat::IDENTITY,
                    45.0,
                ));

                // Mos Eisley Starport
                snapshot.add_node(WorldSnapshotNode::new(
                    100_002,
                    "object/building/tatooine/shared_starport_tatooine.iff",
                    0,
                    Vec3::new(3670.0, 5.0, -4780.0),
                    Quat::from_rotation_y(0.785),
                    80.0,
                ));

                // Lucky Despot Hotel
                snapshot.add_node(WorldSnapshotNode::new(
                    100_003,
                    "object/building/tatooine/shared_hotel_tatooine.iff",
                    0,
                    Vec3::new(3380.0, 5.0, -4620.0),
                    Quat::IDENTITY,
                    50.0,
                ));

                // Mos Eisley Bazaar
                snapshot.add_node(WorldSnapshotNode::new(
                    100_004,
                    "object/building/tatooine/shared_bazaar_tatooine.iff",
                    0,
                    Vec3::new(3450.0, 5.0, -4750.0),
                    Quat::IDENTITY,
                    35.0,
                ));

                // Moisture vaporators around city perimeter
                let vaporator_coords = [
                    Vec3::new(3300.0, 5.0, -4900.0),
                    Vec3::new(3350.0, 5.0, -4920.0),
                    Vec3::new(3700.0, 5.0, -4950.0),
                    Vec3::new(3800.0, 5.0, -4600.0),
                ];
                for (i, pos) in vaporator_coords.into_iter().enumerate() {
                    snapshot.add_node(WorldSnapshotNode::new(
                        100_010 + i as u64,
                        "object/installation/generators/shared_moisture_vaporator.iff",
                        0,
                        pos,
                        Quat::IDENTITY,
                        8.0,
                    ));
                }
            }

            PlanetHub::NabooTheed => {
                // Theed Royal Palace Complex
                snapshot.add_node(WorldSnapshotNode::new(
                    200_001,
                    "object/building/naboo/shared_palace_theed.iff",
                    0,
                    Vec3::new(-5498.0, 25.0, 4478.0),
                    Quat::IDENTITY,
                    150.0,
                ));

                // Theed Starport
                snapshot.add_node(WorldSnapshotNode::new(
                    200_002,
                    "object/building/naboo/shared_starport_naboo.iff",
                    0,
                    Vec3::new(-4860.0, 6.0, 4150.0),
                    Quat::from_rotation_y(1.57),
                    80.0,
                ));

                // Theed Cantina
                snapshot.add_node(WorldSnapshotNode::new(
                    200_003,
                    "object/building/naboo/shared_cantina_naboo.iff",
                    0,
                    Vec3::new(-5000.0, 10.0, 4180.0),
                    Quat::IDENTITY,
                    45.0,
                ));

                // Palace Archway Bridge
                snapshot.add_node(WorldSnapshotNode::new(
                    200_004,
                    "object/building/naboo/shared_bridge_arch_theed.iff",
                    0,
                    Vec3::new(-5300.0, 15.0, 4350.0),
                    Quat::IDENTITY,
                    60.0,
                ));
            }

            PlanetHub::CorelliaCoronet => {
                // Coronet Capitol
                snapshot.add_node(WorldSnapshotNode::new(
                    300_001,
                    "object/building/corellia/shared_capitol_coronet.iff",
                    0,
                    Vec3::new(-210.0, 20.0, -4500.0),
                    Quat::IDENTITY,
                    120.0,
                ));

                // Coronet Starport
                snapshot.add_node(WorldSnapshotNode::new(
                    300_002,
                    "object/building/corellia/shared_starport_coronet.iff",
                    0,
                    Vec3::new(-60.0, 28.0, -4430.0),
                    Quat::from_rotation_y(0.5),
                    85.0,
                ));

                // Coronet Cantina
                snapshot.add_node(WorldSnapshotNode::new(
                    300_003,
                    "object/building/corellia/shared_cantina_corellia.iff",
                    0,
                    Vec3::new(-140.0, 28.0, -4700.0),
                    Quat::IDENTITY,
                    45.0,
                ));
            }
        }

        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canon_hubs_generation() {
        let tatooine = CanonCityGenerator::generate_hub_snapshot(PlanetHub::TatooineMosEisley);
        assert!(tatooine.len() >= 8);
        assert!(
            tatooine
                .nodes
                .iter()
                .any(|n| n.template_name.contains("cantina_tatooine"))
        );
        assert!(
            tatooine
                .nodes
                .iter()
                .any(|n| n.template_name.contains("starport_tatooine"))
        );

        let naboo = CanonCityGenerator::generate_hub_snapshot(PlanetHub::NabooTheed);
        assert!(naboo.len() >= 4);
        assert!(
            naboo
                .nodes
                .iter()
                .any(|n| n.template_name.contains("palace_theed"))
        );

        let corellia = CanonCityGenerator::generate_hub_snapshot(PlanetHub::CorelliaCoronet);
        assert!(corellia.len() >= 3);
        assert!(
            corellia
                .nodes
                .iter()
                .any(|n| n.template_name.contains("capitol_coronet"))
        );
    }
}
