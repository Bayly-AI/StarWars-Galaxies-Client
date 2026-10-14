// crates/swg-client-ui/tests/container_pipeline_tests.rs

use swg_client_ui::{ContainerType, ContainerWindow, DragDropError, DragDropManager, Item};

#[test]
fn test_nested_backpack_exploration_and_drag_drop() {
    let mut main_inventory =
        ContainerWindow::new(1001, ContainerType::Inventory, "Main Inventory", 80, 500);

    // Create a wearable backpack that contains internal items
    let mut backpack_item = Item::new(2001, "item/wearable_backpack.iff", "Field Backpack", 10)
        .with_sub_container(25)
        .with_equip_slot("back");

    // Put items inside the backpack
    backpack_item
        .contained_items
        .push(Item::new(3001, "item/stimpack_b.iff", "Stimpack B", 1).with_stack(10, 20));
    backpack_item
        .contained_items
        .push(Item::new(3002, "item/power_cell.iff", "Fusion Power Cell", 1).with_stack(50, 100));

    main_inventory.add_item(backpack_item).unwrap();
    assert_eq!(main_inventory.items.len(), 1);
    assert!(main_inventory.items[0].is_sub_container());
    assert_eq!(main_inventory.items[0].contained_items.len(), 2);

    // Open nested backpack as its own container window
    let mut backpack_window =
        ContainerWindow::new(2001, ContainerType::Backpack, "Field Backpack", 25, 150);
    for item in main_inventory.items[0].contained_items.clone() {
        backpack_window.add_item(item).unwrap();
    }
    assert_eq!(backpack_window.items.len(), 2);

    // Setup Storage Chest in player house
    let mut house_chest =
        ContainerWindow::new(5001, ContainerType::StorageChest, "Bank Storage", 50, 1000);

    let mut dnd = DragDropManager::new();

    // Drag 15 power cells out of the backpack and drop into the house chest
    dnd.begin_drag(&mut backpack_window, 3002, Some(15))
        .unwrap();
    assert!(dnd.is_dragging());
    assert_eq!(backpack_window.get_item(3002).unwrap().count, 35);

    let move_packet = dnd.drop_into_container(&mut house_chest, None).unwrap();
    assert_eq!(move_packet.item_network_id, 3002);
    assert_eq!(move_packet.source_container_id, 2001);
    assert_eq!(move_packet.target_container_id, 5001);
    assert_eq!(move_packet.split_amount, 15);

    assert_eq!(house_chest.items.len(), 1);
    assert_eq!(house_chest.items[0].count, 15);
}

#[test]
fn test_paperdoll_equipment_movement_and_slot_constraints() {
    let mut inventory =
        ContainerWindow::new(1001, ContainerType::Inventory, "Main Inventory", 80, 500);
    let mut paperdoll = ContainerWindow::new(
        1002,
        ContainerType::Paperdoll,
        "Character Equipment",
        25,
        200,
    );

    // Add weapons and armor to inventory
    let blaster =
        Item::new(4001, "item/weapon_dlt19.iff", "DLT-19 Blaster", 8).with_equip_slot("hold_r");
    let helmet =
        Item::new(4002, "item/armor_helmet.iff", "Composite Helmet", 4).with_equip_slot("head");

    inventory.add_item(blaster).unwrap();
    inventory.add_item(helmet).unwrap();

    let mut dnd = DragDropManager::new();

    // 1. Drag Blaster into "head" slot -> should fail
    dnd.begin_drag(&mut inventory, 4001, None).unwrap();
    let err = dnd.drop_into_paperdoll(&mut paperdoll, "head");
    assert_eq!(
        err,
        Err(DragDropError::InvalidEquipSlot("head".to_string()))
    );

    // 2. Drag Blaster into "hold_r" slot -> should succeed
    let packet = dnd.drop_into_paperdoll(&mut paperdoll, "hold_r").unwrap();
    assert_eq!(packet.target_container_id, 1002);
    assert_eq!(packet.item_network_id, 4001);

    // 3. Drag Helmet into "head" slot -> should succeed
    dnd.begin_drag(&mut inventory, 4002, None).unwrap();
    let packet_helmet = dnd.drop_into_paperdoll(&mut paperdoll, "head").unwrap();
    assert_eq!(packet_helmet.target_container_id, 1002);
    assert_eq!(packet_helmet.item_network_id, 4002);

    assert_eq!(paperdoll.items.len(), 2);
}
