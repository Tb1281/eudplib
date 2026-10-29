//! Frontend vocabulary shared by every Frontend and the LSP.
//!
//! The HIR enums define the semantic values. This module owns the single
//! manifest of Frontend spellings, aliases, and numeric codes for players,
//! comparisons, modifiers, switches, resources, orders, and scores. Frontends
//! derive their constants and decoding from this manifest instead of copying
//! another table, so Python, TypeScript, and the LSP cannot drift apart.
//!
//! [`VocabEntry::code`] is the canonical HIR code. [`VocabEntry::frontend_code`]
//! is the spelling exported by the Python and TypeScript SDK constant tables;
//! it differs from the HIR code only for legacy switch-action aliases.

use crate::hir::{Comparison, Modifier, PlayerId, ResourceType, SwitchAction, SwitchState};

/// A named Frontend spelling for one semantic vocabulary value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocabEntry<T> {
    /// The semantic HIR value.
    pub value: T,
    /// The canonical code stored in the HIR.
    pub code: i32,
    /// The code exported by the Python and TypeScript Frontend constant tables.
    pub frontend_code: i32,
    /// Additional legacy codes accepted when decoding a Frontend value.
    pub alias_codes: &'static [i32],
    /// The canonical Frontend spelling.
    pub name: &'static str,
    /// Alternate Frontend spellings that resolve to the same value.
    pub aliases: &'static [&'static str],
}

impl<T> VocabEntry<T> {
    /// Iterates the canonical spelling followed by every alias.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.name).chain(self.aliases.iter().copied())
    }
}

/// One constant exported by a Frontend SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendConstant {
    /// The Frontend spelling, such as `Player1`, `P1`, or `Force1`.
    pub name: &'static str,
    /// The numeric value bound to that spelling.
    pub code: i32,
}

/// An order type used by the `Order` stock action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderType {
    /// Move to the destination without engaging enemies.
    Move,
    /// Patrol to the destination.
    Patrol,
    /// Attack-move to the destination.
    Attack,
}

/// A score counter used by `Score` and `SetScore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScoreType {
    /// Total score.
    Total,
    /// Units produced.
    Units,
    /// Buildings produced.
    Buildings,
    /// Units and buildings produced.
    UnitsAndBuildings,
    /// Units killed.
    Kills,
    /// Buildings razed.
    Razings,
    /// Units killed and buildings razed.
    KillsAndRazings,
    /// A custom score counter.
    Custom,
}

fn entry_by_value<T: Copy + PartialEq>(
    entries: &'static [VocabEntry<T>],
    value: T,
) -> Option<&'static VocabEntry<T>> {
    entries.iter().find(|entry| entry.value == value)
}

fn push_constant(constants: &mut Vec<FrontendConstant>, name: &'static str, code: i32) {
    if let Some(existing) = constants.iter().find(|constant| constant.name == name) {
        debug_assert_eq!(
            existing.code, code,
            "Frontend spelling {name} has conflicting codes"
        );
        return;
    }
    constants.push(FrontendConstant { name, code });
}

macro_rules! vocabulary {
    (
        $ty:ident, $entries:ident, $code_ty:ty;
        $(
            $variant:ident => code $code:literal, frontend $frontend:literal,
            name $name:literal, aliases [$($alias:literal),*],
            alias_codes [$($alias_code:literal),*]
        ),* $(,)?
    ) => {
        #[doc = concat!("Named Frontend spellings for [`", stringify!($ty), "`].")]
        pub const $entries: &[VocabEntry<$ty>] = &[
            $(
                VocabEntry {
                    value: $ty::$variant,
                    code: $code as i32,
                    frontend_code: $frontend as i32,
                    alias_codes: &[$($alias_code),*],
                    name: $name,
                    aliases: &[$($alias),*],
                }
            ),*
        ];

        impl $ty {
            /// Returns the canonical HIR code for this value.
            pub const fn code(self) -> $code_ty {
                match self {
                    $(Self::$variant => $code,)*
                }
            }

            /// Returns the code exported by the Frontend SDK constant tables.
            pub const fn frontend_code(self) -> $code_ty {
                match self {
                    $(Self::$variant => $frontend,)*
                }
            }

            /// Decodes a Frontend code, accepting canonical, SDK, and alias codes.
            #[allow(clippy::manual_range_patterns)]
            pub const fn from_code(code: $code_ty) -> Option<Self> {
                #[allow(unreachable_patterns)]
                match code {
                    $($code | $frontend $(| $alias_code)* => Some(Self::$variant),)*
                    _ => None,
                }
            }

            /// Iterates the canonical Frontend spellings in manifest order.
            pub fn names() -> impl Iterator<Item = &'static str> {
                $entries.iter().map(|entry| entry.name)
            }

            /// Returns the canonical Frontend spelling, if this value has one.
            pub fn name(self) -> Option<&'static str> {
                entry_by_value($entries, self).map(|entry| entry.name)
            }

            /// Returns the alternate Frontend spellings for this value.
            pub fn aliases(self) -> &'static [&'static str] {
                entry_by_value($entries, self).map_or(&[], |entry| entry.aliases)
            }

            /// Resolves a canonical or alternate Frontend spelling.
            pub fn from_name(name: &str) -> Option<Self> {
                $entries
                    .iter()
                    .find(|entry| entry.name == name || entry.aliases.contains(&name))
                    .map(|entry| entry.value)
            }
        }
    };
}

macro_rules! player_vocabulary {
    (
        $entries:ident;
        $(
            $variant:ident => code $code:literal, frontend $frontend:literal,
            name $name:literal, aliases [$($alias:literal),*],
            alias_codes [$($alias_code:literal),*]
        ),* $(,)?
    ) => {
        /// Named Frontend spellings for [`PlayerId`].
        pub const $entries: &[VocabEntry<PlayerId>] = &[
            $(
                VocabEntry {
                    value: PlayerId::$variant,
                    code: $code as i32,
                    frontend_code: $frontend as i32,
                    alias_codes: &[$($alias_code),*],
                    name: $name,
                    aliases: &[$($alias),*],
                }
            ),*
        ];

        impl PlayerId {
            /// Returns the canonical HIR code for this player selector.
            pub const fn code(self) -> i32 {
                match self {
                    $(Self::$variant => $code,)*
                    Self::Epd(value) => value,
                }
            }

            /// Returns the code exported by the Frontend SDK constant tables.
            pub const fn frontend_code(self) -> i32 {
                self.code()
            }

            /// Decodes every StarCraft player selector.
            ///
            /// All named players and player groups decode to their dedicated
            /// variants. Any other code is a valid EUD player EPD and decodes to
            /// [`PlayerId::Epd`].
            pub const fn from_code(code: i32) -> Self {
                #[allow(unreachable_patterns)]
                match code {
                    $($code | $frontend $(| $alias_code)* => Self::$variant,)*
                    other => Self::Epd(other),
                }
            }

            /// Iterates the canonical Frontend spellings in manifest order.
            pub fn names() -> impl Iterator<Item = &'static str> {
                $entries.iter().map(|entry| entry.name)
            }

            /// Returns the canonical Frontend spelling, if this selector has one.
            pub fn name(self) -> Option<&'static str> {
                entry_by_value($entries, self).map(|entry| entry.name)
            }

            /// Returns the alternate Frontend spellings for this selector.
            pub fn aliases(self) -> &'static [&'static str] {
                entry_by_value($entries, self).map_or(&[], |entry| entry.aliases)
            }

            /// Resolves a canonical or alternate Frontend spelling.
            pub fn from_name(name: &str) -> Option<Self> {
                $entries
                    .iter()
                    .find(|entry| entry.name == name || entry.aliases.contains(&name))
                    .map(|entry| entry.value)
            }

            /// Returns whether this selector addresses a player group.
            pub const fn is_group(self) -> bool {
                matches!(
                    self,
                    Self::Foes
                        | Self::Allies
                        | Self::NeutralPlayers
                        | Self::AllPlayers
                        | Self::Force1
                        | Self::Force2
                        | Self::Force3
                        | Self::Force4
                        | Self::NonAlliedVictoryPlayers
                )
            }
        }
    };
}

player_vocabulary! {
    PLAYER_ENTRIES;
    Player1 => code 0, frontend 0, name "Player1", aliases ["P1"], alias_codes [],
    Player2 => code 1, frontend 1, name "Player2", aliases ["P2"], alias_codes [],
    Player3 => code 2, frontend 2, name "Player3", aliases ["P3"], alias_codes [],
    Player4 => code 3, frontend 3, name "Player4", aliases ["P4"], alias_codes [],
    Player5 => code 4, frontend 4, name "Player5", aliases ["P5"], alias_codes [],
    Player6 => code 5, frontend 5, name "Player6", aliases ["P6"], alias_codes [],
    Player7 => code 6, frontend 6, name "Player7", aliases ["P7"], alias_codes [],
    Player8 => code 7, frontend 7, name "Player8", aliases ["P8"], alias_codes [],
    Player9 => code 8, frontend 8, name "Player9", aliases ["P9"], alias_codes [],
    Player10 => code 9, frontend 9, name "Player10", aliases ["P10"], alias_codes [],
    Player11 => code 10, frontend 10, name "Player11", aliases ["P11"], alias_codes [],
    Player12 => code 11, frontend 11, name "Player12", aliases ["P12"], alias_codes [],
    CurrentPlayer => code 13, frontend 13, name "CurrentPlayer", aliases [], alias_codes [],
    Foes => code 14, frontend 14, name "Foes", aliases [], alias_codes [],
    Allies => code 15, frontend 15, name "Allies", aliases [], alias_codes [],
    NeutralPlayers => code 16, frontend 16, name "NeutralPlayers", aliases [], alias_codes [],
    AllPlayers => code 17, frontend 17, name "AllPlayers", aliases [], alias_codes [],
    Force1 => code 18, frontend 18, name "Force1", aliases [], alias_codes [],
    Force2 => code 19, frontend 19, name "Force2", aliases [], alias_codes [],
    Force3 => code 20, frontend 20, name "Force3", aliases [], alias_codes [],
    Force4 => code 21, frontend 21, name "Force4", aliases [], alias_codes [],
    NonAlliedVictoryPlayers => code 26, frontend 26, name "NonAlliedVictoryPlayers", aliases [], alias_codes [],
}

vocabulary! {
    Comparison, COMPARISON_ENTRIES, u8;
    AtLeast => code 0, frontend 0, name "AtLeast", aliases [], alias_codes [],
    AtMost => code 1, frontend 1, name "AtMost", aliases [], alias_codes [],
    Exactly => code 10, frontend 10, name "Exactly", aliases [], alias_codes [],
}

vocabulary! {
    Modifier, MODIFIER_ENTRIES, u8;
    SetTo => code 7, frontend 7, name "SetTo", aliases [], alias_codes [],
    Add => code 8, frontend 8, name "Add", aliases [], alias_codes [],
    Subtract => code 9, frontend 9, name "Subtract", aliases [], alias_codes [],
}

vocabulary! {
    ResourceType, RESOURCE_ENTRIES, u8;
    Ore => code 0, frontend 0, name "Ore", aliases [], alias_codes [],
    Gas => code 1, frontend 1, name "Gas", aliases [], alias_codes [],
    OreAndGas => code 2, frontend 2, name "OreAndGas", aliases [], alias_codes [],
}

vocabulary! {
    SwitchState, SWITCH_STATE_ENTRIES, u8;
    Set => code 2, frontend 2, name "Set", aliases [], alias_codes [1],
    Cleared => code 3, frontend 3, name "Cleared", aliases [], alias_codes [0],
}

vocabulary! {
    SwitchAction, SWITCH_ACTION_ENTRIES, u8;
    Set => code 4, frontend 2, name "Set", aliases [], alias_codes [],
    Clear => code 5, frontend 5, name "Clear", aliases [], alias_codes [3],
    Toggle => code 6, frontend 6, name "Toggle", aliases [], alias_codes [],
    Random => code 11, frontend 7, name "Random", aliases [], alias_codes [],
}

vocabulary! {
    OrderType, ORDER_ENTRIES, u8;
    Move => code 0, frontend 0, name "Move", aliases [], alias_codes [],
    Patrol => code 1, frontend 1, name "Patrol", aliases [], alias_codes [],
    Attack => code 2, frontend 2, name "Attack", aliases [], alias_codes [],
}

vocabulary! {
    ScoreType, SCORE_ENTRIES, u8;
    Total => code 0, frontend 0, name "Total", aliases [], alias_codes [],
    Units => code 1, frontend 1, name "Units", aliases [], alias_codes [],
    Buildings => code 2, frontend 2, name "Buildings", aliases [], alias_codes [],
    UnitsAndBuildings => code 3, frontend 3, name "UnitsAndBuildings", aliases [], alias_codes [],
    Kills => code 4, frontend 4, name "Kills", aliases [], alias_codes [],
    Razings => code 5, frontend 5, name "Razings", aliases [], alias_codes [],
    KillsAndRazings => code 6, frontend 6, name "KillsAndRazings", aliases [], alias_codes [],
    Custom => code 7, frontend 7, name "Custom", aliases [], alias_codes [],
}

/// Returns every vocabulary constant exported by the Python and TypeScript Frontend SDKs.
///
/// Canonical names and aliases are both included, in manifest order. A spelling
/// that appears in more than one vocabulary (such as `Set`) is emitted once. The
/// `All` unit-count pseudo-value is not part of the vocabulary manifest.
pub fn frontend_constants() -> Vec<FrontendConstant> {
    let mut constants = Vec::new();

    macro_rules! add_entries {
        ($entries:ident) => {
            for entry in $entries {
                push_constant(&mut constants, entry.name, entry.frontend_code);
                for alias in entry.aliases {
                    push_constant(&mut constants, alias, entry.frontend_code);
                }
            }
        };
    }

    add_entries!(PLAYER_ENTRIES);
    add_entries!(COMPARISON_ENTRIES);
    add_entries!(MODIFIER_ENTRIES);
    add_entries!(RESOURCE_ENTRIES);
    add_entries!(SWITCH_STATE_ENTRIES);
    add_entries!(SWITCH_ACTION_ENTRIES);
    add_entries!(ORDER_ENTRIES);
    add_entries!(SCORE_ENTRIES);

    constants
}

/// Resolves one Frontend constant spelling to its exported code.
pub fn frontend_constant(name: &str) -> Option<i32> {
    macro_rules! lookup {
        ($entries:ident) => {
            if let Some(entry) = $entries
                .iter()
                .find(|entry| entry.name == name || entry.aliases.contains(&name))
            {
                return Some(entry.frontend_code);
            }
        };
    }

    lookup!(PLAYER_ENTRIES);
    lookup!(COMPARISON_ENTRIES);
    lookup!(MODIFIER_ENTRIES);
    lookup!(RESOURCE_ENTRIES);
    lookup!(SWITCH_STATE_ENTRIES);
    lookup!(SWITCH_ACTION_ENTRIES);
    lookup!(ORDER_ENTRIES);
    lookup!(SCORE_ENTRIES);
    None
}
