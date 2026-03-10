//! Emulator-based tests for the 99 item stack overflow fix.
//!
//! The bug: When `AddItemToInventory_` splits a stack that exceeds 99 items,
//! it re-enters the search loop at `.addAnotherStackOfItem` to find or create
//! a slot for the remainder.  The search loop can scan past the $FF list
//! terminator into unrelated WRAM, interpreting arbitrary bytes as item ID /
//! quantity pairs and potentially corrupting memory.
//!
//! The fix: After writing 99 to the current slot, jump directly to
//! `.addNewItem` instead of `.addAnotherStackOfItem`.  `.addNewItem`
//! calculates the correct position from the item count and creates the
//! remainder slot safely.  Same byte count (jp = 3 bytes), zero ROM growth.
//!
//! Reference: https://glitchcity.wiki/wiki/99_item_stack_glitch

use pokeyellow_tests::{sym_addr, sym_bank, TestHarness};

const TRAP_ADDR: u16 = 0xC100;
const NOP: u8 = 0x00;
const STOP: u8 = 0x10;

/// Item IDs from the game's item constants.
const GREAT_BALL: u8 = 0x03;
const POTION: u8 = 0x14;

// WRAM addresses (stable, not affected by ROM changes).
const W_NUM_BAG_ITEMS: u16 = 0xD31C;
const W_BAG_ITEMS: u16 = 0xD31D;
const W_CUR_ITEM: u16 = 0xCF90;
const W_ITEM_QUANTITY: u16 = 0xCF95;

const BAG_ITEM_CAPACITY: u8 = 20;

// ─── ROM byte verification ─────────────────────────────────────────

#[test]
fn rom_bytes_split_jumps_to_add_new_item() {
    // After the split (ld a, 99 / ld [hli], a), the jp target should be
    // .addNewItem, NOT .addAnotherStackOfItem.
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    let bank = sym_bank("AddItemToInventory_");
    h.select_rom_bank(bank);

    let add_new_item = sym_addr("AddItemToInventory_.addNewItem");
    let add_another = sym_addr("AddItemToInventory_.addAnotherStackOfItem");
    let _increase = sym_addr("AddItemToInventory_.increaseItemQuantity");

    // The split code is at the end of .increaseItemQuantity:
    //   ...
    //   ld a, 99       ($3E $63)
    //   ld [hli], a    ($22)
    //   jp .addNewItem  ($C3 lo hi)    ← the fix (was: jp .addAnotherStackOfItem)
    //
    // Find the jp instruction: it's the last 3 bytes before .increaseItemQuantityFailed
    let fail_addr = sym_addr("AddItemToInventory_.increaseItemQuantityFailed");
    let jp_addr = fail_addr - 3;

    // Verify it's a jp instruction
    assert_eq!(
        h.read_mem(jp_addr),
        0xC3,
        "expected jp opcode at ${jp_addr:04X}"
    );

    // Read the 16-bit target (little-endian)
    let target_lo = h.read_mem(jp_addr + 1) as u16;
    let target_hi = h.read_mem(jp_addr + 2) as u16;
    let target = target_lo | (target_hi << 8);

    assert_eq!(
        target, add_new_item,
        "jp should target .addNewItem (${add_new_item:04X}), \
         not .addAnotherStackOfItem (${add_another:04X}); got ${target:04X}"
    );

    // Verify the preceding bytes are ld a, 99 / ld [hli], a
    assert_eq!(h.read_mem(jp_addr - 3), 0x3E, "expected ld a, imm ($3E)");
    assert_eq!(h.read_mem(jp_addr - 2), 99, "expected immediate 99");
    assert_eq!(h.read_mem(jp_addr - 1), 0x22, "expected ld [hli], a ($22)");

    // Sanity: .addNewItem should NOT equal .addAnotherStackOfItem
    assert_ne!(
        add_new_item, add_another,
        ".addNewItem and .addAnotherStackOfItem should be different labels"
    );
}

// ─── Behavioral tests ──────────────────────────────────────────────

/// Set up a headless harness ready to call AddItemToInventory_.
fn setup_harness() -> TestHarness {
    let mut h = TestHarness::new_headless();
    h.gb.cpu().set_ime(false);
    h.write_mem(0xFFFF, 0x00); // disable all interrupt sources
    h.gb.set_timer_enabled(false);
    h.gb.set_serial_enabled(false);
    h.gb.set_dma_enabled(false);

    let bank = sym_bank("AddItemToInventory_");
    h.select_rom_bank(bank);
    h.write_mem(0xFFB8, bank); // hLoadedROMBank

    h.write_mem(TRAP_ADDR, NOP);
    h.write_mem(TRAP_ADDR + 1, STOP);

    h
}

/// Write a bag with the given items: &[(item_id, quantity), ...]
fn write_bag(h: &mut TestHarness, items: &[(u8, u8)]) {
    h.write_mem(W_NUM_BAG_ITEMS, items.len() as u8);
    for (i, &(id, qty)) in items.iter().enumerate() {
        let base = W_BAG_ITEMS + (i as u16) * 2;
        h.write_mem(base, id);
        h.write_mem(base + 1, qty);
    }
    // Write terminator
    let term_pos = W_BAG_ITEMS + (items.len() as u16) * 2;
    h.write_mem(term_pos, 0xFF);
}

/// Read the bag contents: returns Vec<(item_id, quantity)>.
fn read_bag(h: &mut TestHarness) -> Vec<(u8, u8)> {
    let count = h.read_mem(W_NUM_BAG_ITEMS) as usize;
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let base = W_BAG_ITEMS + (i as u16) * 2;
        let id = h.read_mem(base);
        let qty = h.read_mem(base + 1);
        items.push((id, qty));
    }
    items
}

/// Call AddItemToInventory_ with the given item and quantity.
/// Returns true if successful (carry flag set).
fn call_add_item(h: &mut TestHarness, item: u8, quantity: u8) -> bool {
    let func = sym_addr("AddItemToInventory_");

    h.write_mem(W_CUR_ITEM, item);
    h.write_mem(W_ITEM_QUANTITY, quantity);

    // Set HL = wNumBagItems (passed as argument)
    h.gb.cpu().h = (W_NUM_BAG_ITEMS >> 8) as u8;
    h.gb.cpu().l = (W_NUM_BAG_ITEMS & 0xFF) as u8;

    h.set_sp(0xDFF0);
    h.push_word(TRAP_ADDR);
    h.set_pc(func);
    h.step_to(TRAP_ADDR);

    // Check carry flag (set = success, clear = failure)
    h.gb.cpu_i().carry()
}

#[test]
fn add_to_stack_of_99_creates_new_slot() {
    // Adding 5 items to a stack of 99 should set current to 99 and create
    // a new slot with 6 (99 + 5 = 104, split to 99 + 5).
    let mut h = setup_harness();

    write_bag(&mut h, &[(POTION, 10), (GREAT_BALL, 99)]);

    let ok = call_add_item(&mut h, GREAT_BALL, 5);
    assert!(ok, "should succeed when bag has room");

    let bag = read_bag(&mut h);
    assert_eq!(bag.len(), 3, "should have 3 item slots");
    assert_eq!(bag[0], (POTION, 10), "first slot unchanged");
    assert_eq!(bag[1], (GREAT_BALL, 99), "second slot capped at 99");
    assert_eq!(
        bag[2],
        (GREAT_BALL, 5),
        "third slot has remainder (104-99=5)"
    );
}

#[test]
fn add_within_99_does_not_split() {
    // Adding items that stay under 99 should just increase the quantity.
    let mut h = setup_harness();

    write_bag(&mut h, &[(GREAT_BALL, 50)]);

    let ok = call_add_item(&mut h, GREAT_BALL, 30);
    assert!(ok, "should succeed");

    let bag = read_bag(&mut h);
    assert_eq!(bag.len(), 1, "should still have 1 slot");
    assert_eq!(bag[0], (GREAT_BALL, 80), "quantity increased to 80");
}

#[test]
fn add_new_item_creates_slot() {
    // Adding a new item type should create a new slot.
    let mut h = setup_harness();

    write_bag(&mut h, &[(POTION, 5)]);

    let ok = call_add_item(&mut h, GREAT_BALL, 3);
    assert!(ok, "should succeed");

    let bag = read_bag(&mut h);
    assert_eq!(bag.len(), 2, "should have 2 slots");
    assert_eq!(bag[0], (POTION, 5));
    assert_eq!(bag[1], (GREAT_BALL, 3));
}

#[test]
fn split_fails_when_bag_full() {
    // With exactly BAG_ITEM_CAPACITY items, adding to a stack of 99 should
    // fail because there's no room for the remainder slot.
    let mut h = setup_harness();

    // Fill bag with 20 different items, last one is Great Ball × 99
    let mut items: Vec<(u8, u8)> = Vec::new();
    for i in 0..19 {
        items.push((POTION + i, 1));
    }
    items.push((GREAT_BALL, 99));
    write_bag(&mut h, &items);

    assert_eq!(
        h.read_mem(W_NUM_BAG_ITEMS),
        BAG_ITEM_CAPACITY,
        "bag should be full"
    );

    let ok = call_add_item(&mut h, GREAT_BALL, 1);
    assert!(!ok, "should fail when bag is full and split is needed");

    // Verify the existing stack was NOT modified
    let bag = read_bag(&mut h);
    assert_eq!(bag[19], (GREAT_BALL, 99), "quantity should be unchanged");
}

#[test]
fn split_with_room_for_one_slot() {
    // Bag has 19 items, Great Ball × 99 at last slot. Adding 1 should
    // succeed: split to 99 + 1, creating slot 20 (exactly at capacity).
    let mut h = setup_harness();

    let mut items: Vec<(u8, u8)> = Vec::new();
    for i in 0..18 {
        items.push((POTION + i, 1));
    }
    items.push((GREAT_BALL, 99));
    write_bag(&mut h, &items);

    assert_eq!(h.read_mem(W_NUM_BAG_ITEMS), 19);

    let ok = call_add_item(&mut h, GREAT_BALL, 1);
    assert!(ok, "should succeed with exactly 1 slot remaining");

    let bag = read_bag(&mut h);
    assert_eq!(bag.len(), 20, "bag should now be full");
    assert_eq!(bag[18], (GREAT_BALL, 99), "original slot stays at 99");
    assert_eq!(
        bag[19],
        (GREAT_BALL, 1),
        "new slot has remainder (100-99=1)"
    );
}

#[test]
fn terminator_preserved_after_split() {
    // After a successful split, the $FF terminator should be correctly
    // placed at the end of the new item list.
    let mut h = setup_harness();

    write_bag(&mut h, &[(GREAT_BALL, 99)]);

    call_add_item(&mut h, GREAT_BALL, 10);

    let count = h.read_mem(W_NUM_BAG_ITEMS) as u16;
    let term_addr = W_BAG_ITEMS + count * 2;
    assert_eq!(
        h.read_mem(term_addr),
        0xFF,
        "terminator should be at the end of the list (${term_addr:04X})"
    );
}

#[test]
fn no_wram_corruption_past_bag_buffer() {
    // Write a sentinel pattern past the bag buffer. After the split,
    // the sentinel should be untouched (no writes past the buffer).
    let mut h = setup_harness();

    // wBagItems has BAG_ITEM_CAPACITY * 2 + 1 = 41 bytes
    // So the buffer goes from W_BAG_ITEMS to W_BAG_ITEMS + 40 (inclusive)
    let buffer_end = W_BAG_ITEMS + (BAG_ITEM_CAPACITY as u16) * 2; // $D31D + 40 = $D345
    let sentinel_addr = buffer_end + 1; // first byte past the buffer

    // Write sentinel
    let sentinel: u8 = 0xBE;
    h.write_mem(sentinel_addr, sentinel);
    h.write_mem(sentinel_addr + 1, sentinel);
    h.write_mem(sentinel_addr + 2, sentinel);

    // Set up bag with 19 items + Great Ball × 99
    let mut items: Vec<(u8, u8)> = Vec::new();
    for i in 0..18 {
        items.push((POTION + i, 1));
    }
    items.push((GREAT_BALL, 99));
    write_bag(&mut h, &items);

    // Add 50 Great Balls — triggers split
    let ok = call_add_item(&mut h, GREAT_BALL, 50);
    assert!(ok, "should succeed");

    // Sentinel bytes should be untouched
    assert_eq!(
        h.read_mem(sentinel_addr),
        sentinel,
        "sentinel at ${sentinel_addr:04X} should be untouched"
    );
    assert_eq!(
        h.read_mem(sentinel_addr + 1),
        sentinel,
        "sentinel at ${:04X} should be untouched",
        sentinel_addr + 1
    );
    assert_eq!(
        h.read_mem(sentinel_addr + 2),
        sentinel,
        "sentinel at ${:04X} should be untouched",
        sentinel_addr + 2
    );
}

#[test]
fn add_to_empty_bag() {
    // Edge case: adding to an empty bag should work.
    let mut h = setup_harness();

    write_bag(&mut h, &[]);

    let ok = call_add_item(&mut h, GREAT_BALL, 5);
    assert!(ok, "should succeed adding to empty bag");

    let bag = read_bag(&mut h);
    assert_eq!(bag.len(), 1);
    assert_eq!(bag[0], (GREAT_BALL, 5));
}
