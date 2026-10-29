use eudkit_core::allocator::{allocate_lir_module, trigger_occupation_map};
use eudkit_core::chk_layout::{
    classic, pack_action, pack_condition, payload, ACTION_COUNT, ACTION_SIZE, CONDITION_COUNT,
    CONDITION_SIZE, PLAYER_COUNT,
};
use eudkit_core::injector::{
    build_trig_section, pack_chk_action, pack_chk_condition, pack_chk_trigger,
    BOOTSTRAP_TRIGGER_COUNT,
};
use eudkit_core::lir::{
    ActField, CondField, FieldRef, LirModule, LirObject, LirTrigger, ObjectId, TriggerRole,
    ValueExpr,
};

fn sample_condition() -> [u8; CONDITION_SIZE] {
    pack_condition(
        0x1122_3344,
        13,
        0x5566_7788,
        0x1234,
        0,  // AtLeast
        15, // Deaths
        1,
        2,
        0x4353,
    )
}

fn sample_action() -> [u8; ACTION_SIZE] {
    pack_action(
        0x0102_0304,
        0x0506_0708,
        0x090A_0B0C,
        0x0D0E_0F10,
        -1,
        -2,
        0x00AA,
        45, // SetDeaths
        7,  // SetTo
        20,
        0x4353,
    )
}

#[test]
fn classic_layout_pack_unpack_round_trip() {
    assert_eq!(classic::TRIGGER_SIZE, 2400);
    assert_eq!(classic::CONDITIONS_OFFSET, 0);
    assert_eq!(classic::ACTIONS_OFFSET, 320);
    assert_eq!(classic::FLAGS_OFFSET, 2368);
    assert_eq!(classic::PLAYERS_OFFSET, 2372);
    assert_eq!(classic::PLAYERS_END, 2400);
    assert_eq!(classic::CONDITION_TYPE_OFFSET, 15);
    assert_eq!(classic::ACTION_TYPE_OFFSET, 26);
    assert_eq!(CONDITION_COUNT, 16);
    assert_eq!(CONDITION_SIZE, 20);
    assert_eq!(ACTION_COUNT, 64);
    assert_eq!(ACTION_SIZE, 32);
    assert_eq!(PLAYER_COUNT, 28);

    let condition = sample_condition();
    let action = sample_action();
    let raw = pack_chk_trigger(&[3, 17], &[condition], &[action], 0xDEAD_BEEF);

    assert_eq!(raw.len(), classic::TRIGGER_SIZE);

    let view = classic::unpack_trigger(&raw);
    assert_eq!(view.flags(), 0xDEAD_BEEF);
    let players = view.players();
    assert_eq!(players[3], 1);
    assert_eq!(players[17], 1);
    assert_eq!(players.iter().filter(|&&p| p != 0).count(), 2);
    assert_eq!(view.condition(0), &condition);
    assert_eq!(view.action(0), &action);
    assert_eq!(view.condition_count(), 1);
    assert_eq!(view.action_count(), 1);

    // Missing slots stay zeroed.
    assert_eq!(view.condition(1), &[0u8; CONDITION_SIZE]);
    assert_eq!(view.action(1), &[0u8; ACTION_SIZE]);
}

#[test]
fn classic_layout_supports_full_slot_counts() {
    let conditions = [sample_condition(); CONDITION_COUNT];
    let actions = [sample_action(); ACTION_COUNT];
    let raw = pack_chk_trigger(&[], &conditions, &actions, 0);

    let view = classic::unpack_trigger(&raw);
    assert_eq!(view.condition_count(), CONDITION_COUNT);
    assert_eq!(view.action_count(), ACTION_COUNT);
    assert_eq!(
        view.condition(CONDITION_COUNT - 1),
        &conditions[CONDITION_COUNT - 1]
    );
    assert_eq!(view.action(ACTION_COUNT - 1), &actions[ACTION_COUNT - 1]);
}

#[test]
fn payload_layout_offsets_and_size() {
    assert_eq!(payload::TRIGGER_SIZE, 2408);
    assert_eq!(payload::PREV_PTR_OFFSET, 0);
    assert_eq!(payload::NEXT_PTR_OFFSET, 4);
    assert_eq!(payload::CONDITIONS_OFFSET, 8);
    assert_eq!(payload::ACTIONS_OFFSET, 328);
    assert_eq!(payload::FLAGS_OFFSET, 2376);
    assert_eq!(payload::CURRENT_ACTION_OFFSET, 2407);
    assert_eq!(payload::TRAILING_UNUSED_BYTES, 27);
    assert_eq!(payload::TRIGGER_SIZE, classic::TRIGGER_SIZE + 8);

    assert_eq!(payload::condition_offset(0), 8);
    assert_eq!(payload::condition_offset(15), 8 + 15 * 20);
    assert_eq!(payload::action_offset(0), 328);
    assert_eq!(payload::action_offset(63), 328 + 63 * 32);

    let condition = sample_condition();
    let action = sample_action();
    let raw = payload::pack_trigger(
        0x0102_0304,
        0x0506_0708,
        &[condition],
        &[action],
        0xCAFE_BABE,
        7,
    );

    assert_eq!(raw.len(), payload::TRIGGER_SIZE);
    let view = payload::unpack_trigger(&raw);
    assert_eq!(view.prev_ptr(), 0x0102_0304);
    assert_eq!(view.next_ptr(), 0x0506_0708);
    assert_eq!(view.flags(), 0xCAFE_BABE);
    assert_eq!(view.current_action(), 7);
    assert_eq!(view.condition(0), &condition);
    assert_eq!(view.action(0), &action);
    assert_eq!(view.condition_count(), 1);
    assert_eq!(view.action_count(), 1);
}

#[test]
fn flags_and_player_offsets_match_the_shared_layout() {
    let raw = pack_chk_trigger(&[0, 27, 28], &[], &[], 0x0102_0304);
    assert_eq!(
        &raw[classic::FLAGS_OFFSET..classic::PLAYERS_OFFSET],
        &0x0102_0304u32.to_le_bytes()
    );
    assert_eq!(raw[classic::PLAYERS_OFFSET], 1);
    assert_eq!(raw[classic::PLAYERS_OFFSET + 27], 1);
    assert_eq!(raw[classic::PLAYERS_OFFSET + 1], 0);
    // Player slot 28 lies past the 28-byte player array and is ignored.
    assert_eq!(
        raw[classic::PLAYERS_OFFSET..classic::PLAYERS_END]
            .iter()
            .filter(|&&byte| byte != 0)
            .count(),
        2
    );

    let raw = payload::pack_trigger(0, 0, &[], &[], 0x0102_0304, 0);
    assert_eq!(
        &raw[payload::FLAGS_OFFSET..payload::FLAGS_OFFSET + 4],
        &0x0102_0304u32.to_le_bytes()
    );
    assert_eq!(raw[payload::CURRENT_ACTION_OFFSET], 0);
    assert!(
        raw[payload::FLAGS_OFFSET + 4..payload::CURRENT_ACTION_OFFSET]
            .iter()
            .all(|&byte| byte == 0)
    );
}

#[test]
fn allocator_injector_and_lir_agree_on_trigger_layout() {
    // The allocator's occupation map is one dword per Payload trigger byte.
    for (conditions, actions) in [(0, 0), (1, 1), (5, 9), (CONDITION_COUNT, ACTION_COUNT)] {
        let occupation = trigger_occupation_map(conditions, actions);
        assert_eq!(occupation.len() * 4, payload::TRIGGER_SIZE);
    }

    // A LIR module with one trigger writes exactly one Payload trigger.
    let lir = LirModule::new(
        "chk_layout_agreement",
        vec![LirObject::Trigger(LirTrigger {
            id: ObjectId(0),
            label: None,
            role: TriggerRole::Normal,
            prev_ptr: ValueExpr::Const(0),
            next_ptr: ValueExpr::Const(0),
            conditions: Vec::new(),
            actions: Vec::new(),
            flags: 0,
        })],
        ObjectId(0),
    );
    let allocated = allocate_lir_module(&lir).expect("single trigger should allocate");
    assert_eq!(allocated.data.len(), payload::TRIGGER_SIZE);
    assert_eq!(allocated.object_offsets[0], 0);

    // The injector packs classic entries of exactly the classic size.
    assert_eq!(
        pack_chk_trigger(&[], &[], &[], 0).len(),
        classic::TRIGGER_SIZE
    );
    assert_eq!(
        pack_chk_condition(0, 0, 0, 0, 0, 0, 0, 0, 0).len(),
        CONDITION_SIZE
    );
    assert_eq!(
        pack_chk_action(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0).len(),
        ACTION_SIZE
    );

    let trig = build_trig_section(0, &[], &[]);
    assert_eq!(trig.len(), BOOTSTRAP_TRIGGER_COUNT * classic::TRIGGER_SIZE);

    // FieldRef offsets are relative to the Payload trigger layout.
    assert_eq!(
        FieldRef::condition(0, 0, CondField::LocId).byte_offset(),
        payload::condition_offset(0) as u32
    );
    assert_eq!(
        FieldRef::action(0, 0, ActField::Player2).byte_offset(),
        (payload::action_offset(0) + 20) as u32
    );
    assert_eq!(
        FieldRef::flags(0).byte_offset(),
        payload::FLAGS_OFFSET as u32
    );
}
