// crates/swg-client-ui/src/container.rs

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ContainerError {
    #[error("Container is full (capacity {0} reached)")]
    ContainerFull(usize),
    #[error("Item volume exceeds container volume capacity")]
    VolumeLimitExceeded,
    #[error("Item {0} not found in container {1}")]
    ItemNotFound(u64, u64),
    #[error("Cannot equip item into incompatible slot '{0}'")]
    IncompatibleEquipSlot(String),
}

/// An inventory item or equipment piece.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub network_id: u64,
    pub template_name: String,
    pub name: String,
    pub icon_path: String,
    pub count: u32,
    pub max_stack: u32,
    pub volume: u32,
    pub allowed_equip_slots: Vec<String>,
    pub equipped_slot: Option<String>,
    pub sub_container_capacity: Option<usize>,
    pub contained_items: Vec<Item>,
}

impl Item {
    #[must_use]
    pub fn new(
        network_id: u64,
        template_name: impl Into<String>,
        name: impl Into<String>,
        volume: u32,
    ) -> Self {
        Self {
            network_id,
            template_name: template_name.into(),
            name: name.into(),
            icon_path: "ui/icons/default_item.dds".to_string(),
            count: 1,
            max_stack: 1,
            volume: volume.max(1),
            allowed_equip_slots: Vec::new(),
            equipped_slot: None,
            sub_container_capacity: None,
            contained_items: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_stack(mut self, count: u32, max_stack: u32) -> Self {
        self.count = count.clamp(1, max_stack);
        self.max_stack = max_stack.max(1);
        self
    }

    #[must_use]
    pub fn with_equip_slot(mut self, slot: impl Into<String>) -> Self {
        self.allowed_equip_slots.push(slot.into());
        self
    }

    #[must_use]
    pub fn with_sub_container(mut self, capacity: usize) -> Self {
        self.sub_container_capacity = Some(capacity);
        self
    }

    #[must_use]
    pub fn is_sub_container(&self) -> bool {
        self.sub_container_capacity.is_some()
    }

    #[must_use]
    pub fn can_equip_in(&self, slot: &str) -> bool {
        self.allowed_equip_slots.iter().any(|s| s == slot)
    }
}

/// Type of item container window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContainerType {
    Inventory,
    Backpack,
    StorageChest,
    CorpseLoot,
    Paperdoll,
}

/// A container window managing grid items, capacity, and nested storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerWindow {
    pub id: u64,
    pub container_type: ContainerType,
    pub title: String,
    pub capacity: usize,
    pub max_volume: u32,
    pub items: Vec<Item>,
}

impl ContainerWindow {
    #[must_use]
    pub fn new(
        id: u64,
        container_type: ContainerType,
        title: impl Into<String>,
        capacity: usize,
        max_volume: u32,
    ) -> Self {
        Self {
            id,
            container_type,
            title: title.into(),
            capacity: capacity.max(1),
            max_volume: max_volume.max(1),
            items: Vec::new(),
        }
    }

    #[must_use]
    pub fn total_volume(&self) -> u32 {
        self.items.iter().map(|it| it.volume * it.count).sum()
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.items.len() >= self.capacity
    }

    /// Adds an item to this container, stacking with matching templates if possible.
    pub fn add_item(&mut self, mut item: Item) -> Result<usize, ContainerError> {
        // Try stacking with existing identical items
        if item.max_stack > 1 {
            for existing in &mut self.items {
                if existing.template_name == item.template_name
                    && existing.count < existing.max_stack
                {
                    let space = existing.max_stack - existing.count;
                    if item.count <= space {
                        existing.count += item.count;
                        return Ok(self.items.len() - 1);
                    } else {
                        existing.count = existing.max_stack;
                        item.count -= space;
                    }
                }
            }
        }

        if self.is_full() {
            return Err(ContainerError::ContainerFull(self.capacity));
        }

        if self.total_volume() + item.volume * item.count > self.max_volume {
            return Err(ContainerError::VolumeLimitExceeded);
        }

        self.items.push(item);
        Ok(self.items.len() - 1)
    }

    /// Removes an item by its network ID.
    pub fn remove_item(&mut self, item_id: u64) -> Option<Item> {
        if let Some(pos) = self.items.iter().position(|it| it.network_id == item_id) {
            Some(self.items.remove(pos))
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_item(&self, item_id: u64) -> Option<&Item> {
        self.items.iter().find(|it| it.network_id == item_id)
    }

    pub fn get_item_mut(&mut self, item_id: u64) -> Option<&mut Item> {
        self.items.iter_mut().find(|it| it.network_id == item_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_stacking_and_capacity() {
        let mut bag =
            ContainerWindow::new(101, ContainerType::Backpack, "Leather Backpack", 2, 100);

        // Add 5 Stimpacks (max stack 10)
        let stim1 = Item::new(1, "item/stimpack_a.iff", "Stimpack A", 1).with_stack(5, 10);
        bag.add_item(stim1).unwrap();
        assert_eq!(bag.items.len(), 1);
        assert_eq!(bag.items[0].count, 5);

        // Add 3 more Stimpacks -> stacks into first slot
        let stim2 = Item::new(2, "item/stimpack_a.iff", "Stimpack A", 1).with_stack(3, 10);
        bag.add_item(stim2).unwrap();
        assert_eq!(bag.items.len(), 1);
        assert_eq!(bag.items[0].count, 8);

        // Add blaster rifle (takes second slot)
        let rifle = Item::new(3, "item/rifle_e11.iff", "E-11 Rifle", 10);
        bag.add_item(rifle).unwrap();
        assert_eq!(bag.items.len(), 2);
        assert!(bag.is_full());

        // Third item fails due to capacity limit
        let helmet = Item::new(4, "item/helmet.iff", "Composite Helmet", 5);
        let err = bag.add_item(helmet);
        assert_eq!(err, Err(ContainerError::ContainerFull(2)));
    }
}
