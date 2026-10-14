// crates/swg-client-ui/src/drag_drop.rs

use crate::container::{ContainerError, ContainerWindow, Item};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DragDropError {
    #[error("No active drag operation in progress")]
    NoActiveDrag,
    #[error("Item already being dragged")]
    DragAlreadyActive,
    #[error("Container error: {0}")]
    Container(#[from] ContainerError),
    #[error("Cannot equip item into slot '{0}'")]
    InvalidEquipSlot(String),
}

/// Active drag state representing an item currently held on the mouse cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragSource {
    pub source_container_id: u64,
    pub item: Item,
    pub split_count: Option<u32>,
}

/// Authoritative client-to-server move packet generated upon successful drop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMovePacket {
    pub item_network_id: u64,
    pub source_container_id: u64,
    pub target_container_id: u64,
    pub target_slot: i32,
    pub split_amount: u32,
}

/// Manages interactive drag-and-drop mechanics across container windows and equipment slots.
#[derive(Debug, Default)]
pub struct DragDropManager {
    pub active_drag: Option<DragSource>,
}

impl DragDropManager {
    #[must_use]
    pub fn new() -> Self {
        Self { active_drag: None }
    }

    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.active_drag.is_some()
    }

    /// Begins dragging an item from a container, optionally splitting off a partial stack.
    pub fn begin_drag(
        &mut self,
        source_container: &mut ContainerWindow,
        item_id: u64,
        split_amount: Option<u32>,
    ) -> Result<(), DragDropError> {
        if self.active_drag.is_some() {
            return Err(DragDropError::DragAlreadyActive);
        }

        let container_id = source_container.id;
        let item = source_container
            .get_item_mut(item_id)
            .ok_or(ContainerError::ItemNotFound(item_id, container_id))?;

        let dragged_item = if let Some(split) = split_amount {
            if split < item.count && split > 0 {
                item.count -= split;
                let mut split_item = item.clone();
                split_item.count = split;
                split_item
            } else {
                source_container.remove_item(item_id).unwrap()
            }
        } else {
            source_container.remove_item(item_id).unwrap()
        };

        self.active_drag = Some(DragSource {
            source_container_id: source_container.id,
            item: dragged_item,
            split_count: split_amount,
        });

        Ok(())
    }

    /// Cancels active drag, returning held item back to its origin container.
    pub fn cancel_drag(&mut self, source_container: &mut ContainerWindow) {
        if let Some(drag) = self.active_drag.take() {
            let _ = source_container.add_item(drag.item);
        }
    }

    /// Completes drop into a target container window, generating an authoritative server move packet.
    pub fn drop_into_container(
        &mut self,
        target_container: &mut ContainerWindow,
        target_slot: Option<usize>,
    ) -> Result<ServerMovePacket, DragDropError> {
        let drag = self.active_drag.take().ok_or(DragDropError::NoActiveDrag)?;
        let item_id = drag.item.network_id;
        let source_id = drag.source_container_id;
        let split_amount = drag.split_count.unwrap_or(0);

        // Add to target container
        let slot_idx = target_container.add_item(drag.item)?;

        Ok(ServerMovePacket {
            item_network_id: item_id,
            source_container_id: source_id,
            target_container_id: target_container.id,
            target_slot: target_slot.map_or(slot_idx as i32, |s| s as i32),
            split_amount,
        })
    }

    /// Completes drop into a paperdoll equipment slot.
    pub fn drop_into_paperdoll(
        &mut self,
        paperdoll: &mut ContainerWindow,
        equip_slot: &str,
    ) -> Result<ServerMovePacket, DragDropError> {
        let drag = self
            .active_drag
            .as_ref()
            .ok_or(DragDropError::NoActiveDrag)?;

        if !drag.item.can_equip_in(equip_slot) {
            return Err(DragDropError::InvalidEquipSlot(equip_slot.to_string()));
        }

        let mut drag = self.active_drag.take().unwrap();
        drag.item.equipped_slot = Some(equip_slot.to_string());

        let item_id = drag.item.network_id;
        let source_id = drag.source_container_id;

        paperdoll.add_item(drag.item)?;

        Ok(ServerMovePacket {
            item_network_id: item_id,
            source_container_id: source_id,
            target_container_id: paperdoll.id,
            target_slot: -1,
            split_amount: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::ContainerType;

    #[test]
    fn test_drag_drop_between_containers_and_split() {
        let mut inventory = ContainerWindow::new(1, ContainerType::Inventory, "Inventory", 80, 500);
        let mut chest =
            ContainerWindow::new(2, ContainerType::StorageChest, "Storage Chest", 50, 1000);

        // Put a stack of 20 power cells into inventory
        let cells = Item::new(101, "item/power_cell.iff", "Power Cell", 1).with_stack(20, 50);
        inventory.add_item(cells).unwrap();

        let mut dnd = DragDropManager::new();

        // Split drag 5 power cells from inventory
        dnd.begin_drag(&mut inventory, 101, Some(5)).unwrap();
        assert!(dnd.is_dragging());
        assert_eq!(inventory.get_item(101).unwrap().count, 15);

        // Drop into chest
        let packet = dnd.drop_into_container(&mut chest, None).unwrap();
        assert!(!dnd.is_dragging());
        assert_eq!(packet.item_network_id, 101);
        assert_eq!(packet.source_container_id, 1);
        assert_eq!(packet.target_container_id, 2);
        assert_eq!(packet.split_amount, 5);

        assert_eq!(chest.items.len(), 1);
        assert_eq!(chest.items[0].count, 5);
    }

    #[test]
    fn test_paperdoll_equip_validation() {
        let mut inventory = ContainerWindow::new(1, ContainerType::Inventory, "Inventory", 80, 500);
        let mut paperdoll =
            ContainerWindow::new(99, ContainerType::Paperdoll, "Equipment", 20, 100);

        // Armor that can only equip in "chest"
        let armor =
            Item::new(201, "item/armor_chest.iff", "Composite Chest", 10).with_equip_slot("chest");
        inventory.add_item(armor).unwrap();

        let mut dnd = DragDropManager::new();
        dnd.begin_drag(&mut inventory, 201, None).unwrap();

        // Dropping into helmet slot fails
        let err = dnd.drop_into_paperdoll(&mut paperdoll, "helmet");
        assert_eq!(
            err,
            Err(DragDropError::InvalidEquipSlot("helmet".to_string()))
        );

        // Dropping into chest slot succeeds
        let packet = dnd.drop_into_paperdoll(&mut paperdoll, "chest").unwrap();
        assert_eq!(packet.target_container_id, 99);
        assert_eq!(paperdoll.items[0].equipped_slot.as_deref(), Some("chest"));
    }
}
