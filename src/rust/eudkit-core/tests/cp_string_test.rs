use eudkit_core::hir::{epd, CURRENT_PLAYER_ADDR};
use eudkit_core::lir::{emit_cpstring_residue_stream, ValueExpr};

#[test]
fn cpstring_residue_stride_streaming_groups_mod12_and_respects_n_plus_11_bound() {
    let dest_epd = ValueExpr::Const(100_000);

    // Test case 1: 48 dwords (192 bytes), exactly 4 dwords per residue class (k = 0..4)
    let dwords: Vec<u32> = (0..48).map(|i| 0x1000_0000 + i).collect();
    let actions = emit_cpstring_residue_stream(dest_epd, &dwords);

    let mut write_actions = 0;
    let mut advance_actions = 0;
    let mut current_residue = 0;
    let mut current_k_for_residue = 0;

    for act in &actions {
        if act.player1 == ValueExpr::Const(13) {
            // Write action: SetDeaths(CurrentPlayer, SetTo, dword, unit=k)
            write_actions += 1;
            assert_eq!(act.acttype, 45); // SetDeaths
            assert_eq!(act.amount, 7); // SetTo
            assert_eq!(act.flags, 20);

            // Unit k must match the expected k for this residue class
            assert_eq!(act.unitid, current_k_for_residue as u16);
            let expected_dword = 0x1000_0000 + (current_k_for_residue * 12 + current_residue);
            assert_eq!(act.player2, ValueExpr::Const(expected_dword as i32));

            current_k_for_residue += 1;
        } else if act.player1 == ValueExpr::Const(epd(CURRENT_PLAYER_ADDR)) {
            // Advance action: SetDeaths(EPD(0x6509B0), Add, delta, unit=0)
            advance_actions += 1;
            assert_eq!(act.acttype, 45); // SetDeaths
            assert_eq!(act.amount, 8); // Add
            assert_eq!(act.unitid, 0);

            // Moves to the next residue class
            current_residue += 1;
            current_k_for_residue = 0;
        } else {
            panic!("Unexpected action in CPString residue stream: {:?}", act);
        }
    }

    assert_eq!(write_actions, 48, "Must emit exactly N=48 write actions");
    assert_eq!(advance_actions, 11, "Must advance 0x6509B0 at most 11 times");
    assert!(
        actions.len() <= 48 + 11,
        "Total actions ({}) must be <= N + 11 (59)",
        actions.len()
    );
    assert!(
        actions.len() <= 64,
        "48 dwords (192 bytes) must fit within a single 64-action trigger without looping"
    );
}

#[test]
fn cpstring_residue_stride_streaming_handles_arbitrary_lengths_and_empty_classes() {
    let dest_epd = ValueExpr::Const(50_000);

    // Test case 2: 17 dwords. Residues 0..5 have 2 dwords (k=0,1), residues 5..12 have 1 dword (k=0)
    let dwords: Vec<u32> = (0..17).map(|i| 0xA000_0000 + i).collect();
    let actions = emit_cpstring_residue_stream(dest_epd, &dwords);

    let write_actions = actions
        .iter()
        .filter(|a| a.player1 == ValueExpr::Const(13))
        .count();
    let advance_actions = actions
        .iter()
        .filter(|a| a.player1 == ValueExpr::Const(epd(CURRENT_PLAYER_ADDR)))
        .count();

    assert_eq!(write_actions, 17);
    assert!(advance_actions <= 11);
    assert!(actions.len() <= 17 + 11);
}
