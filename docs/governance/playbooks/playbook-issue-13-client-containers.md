# Playbook: Issue 13 — UI Containers: Drag-and-Drop Grid Inventory, Backpacks, and Container Windows (`swg-client-ui`)

> **Target Issue:** Bayly-AI/StarWars-Galaxies-Client#13  
> **Branch:** `enhancement/13-drag-drop-grid-inventory`  
> **Status:** Implementation Playbook

---

## 1. Objectives & Scope

Implement Pre-CU grid inventory, nested backpacks, and drag-and-drop item movement in `crates/swg-client-ui`:

1. **Inventory & Container Models (`container.rs`):**
   - Hierarchical container structures: main player inventory, nested backpacks, world storage chests, factory hoppers, and corpse loot containers.
   - `Item` metadata: Network ID, template path, volume, stack size, and equipment slot tags.
   - Capacity tracking and volume validation ($V_{\text{total}} \le V_{\text{max}}$).

2. **Drag-and-Drop Item Movement (`drag_drop.rs`):**
   - Interactive drag operations: picking up item stacks or partial stacks ($N_{\text{split}} < N_{\text{current}}$).
   - Drop targets: grid slots, nested sub-containers (backpacks), paperdoll equipment sockets, and world terrain (ground drops).
   - Produces authoritative client-to-server move packet payloads (`ServerMovePacket`) dispatching move requests over SOE UDP netcode.

3. **Paperdoll Equipment Sockets:**
   - Validates equip conditions (e.g. chest armor cannot be dragged to helmet socket).
   - Equipping and un-equipping items between inventory grids and worn character slots.

---

## 2. Acceptance Criteria & Quality Gates

- [x] Supports nested container exploration (opening backpacks inside inventory).
- [x] Allows dragging items between container windows and paperdoll slots with volume/capacity enforcement.
- [x] Accurately splits stacks and merges matching items.
- [x] Generates authoritative server move request packets.
- [x] Full workspace tests pass (`cargo test --workspace`) and zero clippy warnings (`cargo clippy --workspace -- -D warnings`).
- [x] `hath0r preflight run` passes all quality gates.
