use eudkit_core::hir::{Comparison, Modifier, PlayerId, ResourceType, SwitchAction, SwitchState};
use eudkit_core::vocabulary::{
    frontend_constant, frontend_constants, OrderType, ScoreType, COMPARISON_ENTRIES,
    MODIFIER_ENTRIES, ORDER_ENTRIES, PLAYER_ENTRIES, RESOURCE_ENTRIES, SCORE_ENTRIES,
    SWITCH_ACTION_ENTRIES, SWITCH_STATE_ENTRIES,
};
use std::collections::HashMap;

#[test]
fn player_groups_round_trip_through_from_code() {
    for entry in PLAYER_ENTRIES {
        assert_eq!(
            PlayerId::from_code(entry.code),
            entry.value,
            "{} must decode from its canonical code",
            entry.name
        );
        assert_eq!(entry.value.code(), entry.code, "{}", entry.name);
        assert_eq!(
            entry.value.frontend_code(),
            entry.frontend_code,
            "{}",
            entry.name
        );
        assert_eq!(entry.value.name(), Some(entry.name));
        assert_eq!(PlayerId::from_name(entry.name), Some(entry.value));
        for alias in entry.aliases {
            assert_eq!(PlayerId::from_name(alias), Some(entry.value));
        }
    }
}

#[test]
fn force1_and_every_player_group_decode_without_becoming_epd() {
    let groups = [
        (14, PlayerId::Foes),
        (15, PlayerId::Allies),
        (16, PlayerId::NeutralPlayers),
        (17, PlayerId::AllPlayers),
        (18, PlayerId::Force1),
        (19, PlayerId::Force2),
        (20, PlayerId::Force3),
        (21, PlayerId::Force4),
        (26, PlayerId::NonAlliedVictoryPlayers),
    ];

    for (code, expected) in groups {
        assert_eq!(PlayerId::from_code(code), expected);
        assert_eq!(PlayerId::from_player_slot(code as u8), expected);
        assert!(expected.is_group(), "{expected} must be a group");
        assert!(!matches!(PlayerId::from_code(code), PlayerId::Epd(_)));
    }

    assert_eq!(PlayerId::from_code(18), PlayerId::Force1);
    assert_eq!(PlayerId::from_name("Force1"), Some(PlayerId::Force1));
    assert!(!PlayerId::Player1.is_group());
    assert_eq!(PlayerId::from_code(12), PlayerId::Epd(12));
}

#[test]
fn invalid_semantic_codes_are_rejected() {
    assert_eq!(Comparison::from_code(2), None);
    assert_eq!(Modifier::from_code(0), None);
    assert_eq!(ResourceType::from_code(3), None);
    assert_eq!(SwitchState::from_code(4), None);
    assert_eq!(SwitchAction::from_code(0), None);
    assert_eq!(SwitchAction::from_code(8), None);
    assert_eq!(OrderType::from_code(3), None);
    assert_eq!(ScoreType::from_code(8), None);

    assert_eq!(Comparison::from_code(10), Some(Comparison::Exactly));
    assert_eq!(Modifier::from_code(9), Some(Modifier::Subtract));
    assert_eq!(ResourceType::from_code(2), Some(ResourceType::OreAndGas));
}

#[test]
fn switch_aliases_decode_to_the_same_semantic_values() {
    for code in [2, 4] {
        assert_eq!(SwitchAction::from_code(code), Some(SwitchAction::Set));
    }
    for code in [3, 5] {
        assert_eq!(SwitchAction::from_code(code), Some(SwitchAction::Clear));
    }
    assert_eq!(SwitchAction::from_code(6), Some(SwitchAction::Toggle));
    for code in [7, 11] {
        assert_eq!(SwitchAction::from_code(code), Some(SwitchAction::Random));
    }

    for code in [1, 2] {
        assert_eq!(SwitchState::from_code(code), Some(SwitchState::Set));
    }
    for code in [0, 3] {
        assert_eq!(SwitchState::from_code(code), Some(SwitchState::Cleared));
    }

    assert_eq!(SwitchAction::Set.code(), 4);
    assert_eq!(SwitchAction::Set.frontend_code(), 2);
    assert_eq!(SwitchAction::Random.code(), 11);
    assert_eq!(SwitchAction::Random.frontend_code(), 7);
}

#[test]
fn frontend_constants_have_one_value_per_spelling() {
    let constants = frontend_constants();
    let mut seen = HashMap::new();
    for constant in &constants {
        if let Some(previous) = seen.insert(constant.name, constant.code) {
            assert_eq!(
                previous, constant.code,
                "{} has conflicting Frontend codes",
                constant.name
            );
        }
        assert_eq!(frontend_constant(constant.name), Some(constant.code));
    }

    assert_eq!(frontend_constant("Force1"), Some(18));
    assert_eq!(frontend_constant("P1"), Some(0));
    assert_eq!(frontend_constant("Set"), Some(2));
    assert_eq!(frontend_constant("Clear"), Some(5));
    assert_eq!(frontend_constant("Random"), Some(7));
    assert_eq!(frontend_constant("Move"), Some(0));
    assert_eq!(frontend_constant("KillsAndRazings"), Some(6));
    assert_eq!(frontend_constant("NotAVocabularyName"), None);
}

#[test]
fn every_manifest_table_is_internally_consistent() {
    macro_rules! check_table {
        ($entries:ident) => {
            for entry in $entries {
                assert_eq!(
                    entry.value.code(),
                    entry.code as u8,
                    "{} canonical code",
                    entry.name
                );
                assert_eq!(
                    entry.value.frontend_code(),
                    entry.frontend_code as u8,
                    "{} Frontend code",
                    entry.name
                );
                assert_eq!(entry.value.name(), Some(entry.name));
                assert_eq!(entry.value.aliases(), entry.aliases);
            }
        };
    }

    check_table!(COMPARISON_ENTRIES);
    check_table!(MODIFIER_ENTRIES);
    check_table!(RESOURCE_ENTRIES);
    check_table!(SWITCH_STATE_ENTRIES);
    check_table!(SWITCH_ACTION_ENTRIES);
    check_table!(ORDER_ENTRIES);
    check_table!(SCORE_ENTRIES);
}

#[test]
fn typescript_sdk_constants_match_the_core_manifest() {
    let source = include_str!("../../../../packages/ts-sdk/src/index.ts");
    let mut sdk = HashMap::new();

    for line in source.lines() {
        let Some(rest) = line.strip_prefix("export const ") else {
            continue;
        };
        let Some((name, value)) = rest.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim().trim_end_matches(';').trim();
        let Ok(code) = value.parse::<i32>() else {
            continue;
        };
        sdk.insert(name, code);
    }

    for constant in frontend_constants() {
        assert_eq!(
            sdk.get(constant.name).copied(),
            Some(constant.code),
            "TypeScript SDK constant {} drifted from the core vocabulary",
            constant.name
        );
    }
}

#[test]
fn python_types_constants_are_generated_from_the_core_manifest() {
    let source = include_str!("../../../../src/eudkit/types.py");
    assert!(
        source.contains("_rust_eudkit.vocabulary_constants()"),
        "types.py must consume the PyO3 vocabulary manifest"
    );
    assert!(source.contains("All = 0"));

    let mut python = HashMap::new();
    for line in source.lines() {
        let Some((_, rest)) = line.split_once("= _VOCABULARY[\"") else {
            continue;
        };
        let Some((name, _)) = rest.split_once('"') else {
            continue;
        };
        python.insert(name, ());
    }

    for constant in frontend_constants() {
        assert!(
            python.contains_key(constant.name),
            "Python types.py is missing the Frontend spelling {}",
            constant.name
        );
    }

    let mut expected: Vec<&str> = frontend_constants()
        .into_iter()
        .map(|constant| constant.name)
        .collect();
    expected.sort_unstable();
    let mut actual: Vec<&str> = python.keys().copied().collect();
    actual.sort_unstable();
    assert_eq!(actual, expected);
}
