//! HIR (High-Level IR) contract: structured functions, virtual variables,
//! control flow, and game intents before lowering to StarCraft trigger
//! structures.
//!
//! This module owns the IR types, enums, accessors, and small constructors.
//! Semantic transformations live in sibling modules:
//!
//! - [`optimize`] folds constants and simplifies expressions.
//! - [`format`] parses print format templates.
//! - [`reactive`] lowers Reactive Rules.
//! - [`dump`] renders the debug presentation used by `dump-ir`.
//! - [`builder`] constructs HIR incrementally for Frontends.

pub mod builder;
pub mod dump;
pub mod format;
pub mod optimize;
pub mod reactive;

pub use builder::{HirBuilder, OpenControlFrame};
pub use format::parse_format_template;
pub use reactive::lower_reactive_rules;

use crate::lir::FieldRef;
use std::fmt;

pub const DEATHS_BASE_ADDR: u32 = 0x0058_A364;
pub const CURRENT_PLAYER_ADDR: u32 = 0x0065_09B0;
pub const PTS_BASE_ADDR: u32 = 0x0051_A280;
pub const GAME_TICK_ADDR: u32 = 0x0057_F23C;
pub const MRGN_RUNTIME_ADDR: u32 = 0x0058_DC60;
pub const STRX_BASE_ADDR: u32 = 0x1919_43C8;
pub const END_OF_TRIGGERS_SENTINEL: u32 = 0x8000_0000;

#[inline]
pub const fn epd(address: u32) -> i32 {
    ((address as i64) - (DEATHS_BASE_ADDR as i64)).div_euclid(4) as i32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FuncId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ByteBufId(pub u32);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct VarId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HirVar {
    pub id: VarId,
    pub name: String,
    pub init_val: i32,
}

impl HirVar {
    pub fn new(name: impl Into<String>, init_val: i32) -> Self {
        Self {
            id: VarId(0),
            name: name.into(),
            init_val,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlayerId {
    Player1,
    Player2,
    Player3,
    Player4,
    Player5,
    Player6,
    Player7,
    Player8,
    Player9,
    Player10,
    Player11,
    Player12,
    CurrentPlayer,
    Foes,
    Allies,
    NeutralPlayers,
    AllPlayers,
    Force1,
    Force2,
    Force3,
    Force4,
    NonAlliedVictoryPlayers,
    Epd(i32),
}

impl PlayerId {
    /// Decodes a zero-based player slot.
    ///
    /// This is a compatibility wrapper over [`PlayerId::from_code`], which also
    /// decodes every player group such as `Force1` and `Foes`.
    pub const fn from_player_slot(slot: u8) -> Self {
        Self::from_code(slot as i32)
    }

    /// Returns the canonical HIR code for this player selector.
    pub const fn encode(self) -> i32 {
        self.code()
    }
}

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Player1 => write!(f, "P1"),
            Self::Player2 => write!(f, "P2"),
            Self::Player3 => write!(f, "P3"),
            Self::Player4 => write!(f, "P4"),
            Self::Player5 => write!(f, "P5"),
            Self::Player6 => write!(f, "P6"),
            Self::Player7 => write!(f, "P7"),
            Self::Player8 => write!(f, "P8"),
            Self::Player9 => write!(f, "P9"),
            Self::Player10 => write!(f, "P10"),
            Self::Player11 => write!(f, "P11"),
            Self::Player12 => write!(f, "P12"),
            Self::CurrentPlayer => write!(f, "CurrentPlayer"),
            Self::Foes => write!(f, "Foes"),
            Self::Allies => write!(f, "Allies"),
            Self::NeutralPlayers => write!(f, "NeutralPlayers"),
            Self::AllPlayers => write!(f, "AllPlayers"),
            Self::Force1 => write!(f, "Force1"),
            Self::Force2 => write!(f, "Force2"),
            Self::Force3 => write!(f, "Force3"),
            Self::Force4 => write!(f, "Force4"),
            Self::NonAlliedVictoryPlayers => write!(f, "NonAlliedVictoryPlayers"),
            Self::Epd(v) => write!(f, "EPD({v})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Comparison {
    AtLeast,
    AtMost,
    Exactly,
}

impl Comparison {
    /// Returns the canonical HIR code for this comparison.
    pub const fn encode(self) -> u8 {
        self.code()
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AtLeast => write!(f, "AtLeast"),
            Self::AtMost => write!(f, "AtMost"),
            Self::Exactly => write!(f, "Exactly"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SwitchState {
    Set,
    Cleared,
}

impl SwitchState {
    /// Returns the canonical HIR code for this switch state.
    pub const fn encode(self) -> u8 {
        self.code()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SwitchAction {
    Set,
    Clear,
    Toggle,
    Random,
}

impl SwitchAction {
    /// Returns the canonical HIR code for this switch action.
    pub const fn encode(self) -> u8 {
        self.code()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Modifier {
    SetTo,
    Add,
    Subtract,
}

impl Modifier {
    /// Returns the canonical HIR code for this modifier.
    pub const fn encode(self) -> u8 {
        self.code()
    }
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetTo => write!(f, "SetTo"),
            Self::Add => write!(f, "Add"),
            Self::Subtract => write!(f, "Subtract"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResourceType {
    Ore,
    Gas,
    OreAndGas,
}

impl ResourceType {
    /// Returns the canonical HIR code for this resource type.
    pub const fn encode(self) -> u8 {
        self.code()
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ore => write!(f, "Ore"),
            Self::Gas => write!(f, "Gas"),
            Self::OreAndGas => write!(f, "OreAndGas"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirValue {
    Const(i32),
    ByteBufAddress(ByteBufId, u32),
    ByteBufEpd(ByteBufId, i32),
    VarAddress(VarId),
    VarEpd(VarId),
    FieldAddress(FieldRef),
    FieldEpd(FieldRef),
}

impl fmt::Display for HirValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(c) => write!(f, "{c}"),
            Self::ByteBufAddress(id, off) => write!(f, "addr(@buf{}+{})", id.0, off),
            Self::ByteBufEpd(id, off) => write!(f, "epd(@buf{}+{})", id.0, off),
            Self::VarAddress(id) => write!(f, "addr(@v{})", id.0),
            Self::VarEpd(id) => write!(f, "epd(@v{})", id.0),
            Self::FieldAddress(fr) => write!(f, "addr({fr})"),
            Self::FieldEpd(fr) => write!(f, "epd({fr})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HirUnaryOp {
    Neg,
    BitNot,
}

impl fmt::Display for HirUnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Neg => write!(f, "-"),
            Self::BitNot => write!(f, "~"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HirBinOp {
    Add,
    Sub,
    SubSat,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl fmt::Display for HirBinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::SubSat => write!(f, "-_sat"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "//"),
            Self::Rem => write!(f, "%"),
            Self::BitAnd => write!(f, "&"),
            Self::BitOr => write!(f, "|"),
            Self::BitXor => write!(f, "^"),
            Self::Shl => write!(f, "<<"),
            Self::Shr => write!(f, ">>"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirExpr {
    Const(i32),
    Var(VarId),
    UnaryOp {
        op: HirUnaryOp,
        operand: Box<HirExpr>,
    },
    BinOp {
        op: HirBinOp,
        lhs: Box<HirExpr>,
        rhs: Box<HirExpr>,
    },
}

impl HirExpr {
    pub fn substitute_vars(&self, subst: &std::collections::HashMap<VarId, HirExpr>) -> Self {
        match self {
            Self::Const(c) => Self::Const(*c),
            Self::Var(v) => {
                if let Some(replacement) = subst.get(v) {
                    replacement.clone()
                } else {
                    Self::Var(*v)
                }
            }
            Self::UnaryOp { op, operand } => Self::UnaryOp {
                op: *op,
                operand: Box::new(operand.substitute_vars(subst)),
            },
            Self::BinOp { op, lhs, rhs } => Self::BinOp {
                op: *op,
                lhs: Box::new(lhs.substitute_vars(subst)),
                rhs: Box::new(rhs.substitute_vars(subst)),
            },
        }
    }
}

impl fmt::Display for HirExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(c) => write!(f, "{c}"),
            Self::Var(v) => write!(f, "@v{}", v.0),
            Self::UnaryOp { op, operand } => write!(f, "({op}{operand})"),
            Self::BinOp { op, lhs, rhs } => write!(f, "({lhs} {op} {rhs})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StockCondition {
    Always,
    Never,
    Deaths {
        player: PlayerId,
        comparison: Comparison,
        amount: u32,
        unit: u16,
    },
    DeathsX {
        player: PlayerId,
        comparison: Comparison,
        amount: u32,
        unit: u16,
        mask: u32,
    },
    DeathsDynamic {
        player: HirExpr,
        comparison: Comparison,
        amount: HirExpr,
        unit: u16,
        mask: Option<u32>,
    },
    Memory {
        address: u32,
        comparison: Comparison,
        value: u32,
    },
    MemoryX {
        address: u32,
        comparison: Comparison,
        value: u32,
        mask: u32,
    },
    MemoryDynamic {
        address: u32,
        comparison: Comparison,
        value: HirExpr,
        mask: Option<u32>,
    },
    VarCmp {
        var: VarId,
        comparison: Comparison,
        value: u32,
        mask: Option<u32>,
    },
    ExprCmp {
        lhs: HirExpr,
        comparison: Comparison,
        rhs: HirExpr,
    },
    Accumulate {
        player: PlayerId,
        comparison: Comparison,
        amount: u32,
        resource: ResourceType,
    },
    Command {
        player: PlayerId,
        comparison: Comparison,
        amount: u32,
        unit: u16,
    },
    Bring {
        player: PlayerId,
        comparison: Comparison,
        amount: u32,
        unit: u16,
        location: u32,
    },
    Switch {
        switch: u8,
        state: SwitchState,
    },
    ElapsedTime {
        comparison: Comparison,
        seconds: u32,
    },
    Score {
        player: PlayerId,
        score_type: u8,
        comparison: Comparison,
        amount: u32,
    },
    IsUserCp,
}

fn invert_cmp_amount(comparison: Comparison, amount: u32) -> Option<(Comparison, u32)> {
    match comparison {
        Comparison::AtLeast if amount >= 1 => Some((Comparison::AtMost, amount - 1)),
        Comparison::AtMost if amount < u32::MAX => Some((Comparison::AtLeast, amount + 1)),
        Comparison::Exactly if amount == 0 => Some((Comparison::AtLeast, 1)),
        Comparison::Exactly if amount == u32::MAX => Some((Comparison::AtMost, u32::MAX - 1)),
        _ => None,
    }
}

impl StockCondition {
    pub fn invert(&self) -> Option<Self> {
        match *self {
            Self::Always => Some(Self::Never),
            Self::Never => Some(Self::Always),
            Self::Switch { switch, state } => Some(Self::Switch {
                switch,
                state: match state {
                    SwitchState::Set => SwitchState::Cleared,
                    SwitchState::Cleared => SwitchState::Set,
                },
            }),
            Self::Deaths {
                player,
                comparison,
                amount,
                unit,
            } => invert_cmp_amount(comparison, amount).map(|(cmp, amt)| Self::Deaths {
                player,
                comparison: cmp,
                amount: amt,
                unit,
            }),
            Self::DeathsX {
                player,
                comparison,
                amount,
                unit,
                mask,
            } => invert_cmp_amount(comparison, amount).map(|(cmp, amt)| Self::DeathsX {
                player,
                comparison: cmp,
                amount: amt,
                unit,
                mask,
            }),
            Self::Memory {
                address,
                comparison,
                value,
            } => invert_cmp_amount(comparison, value).map(|(cmp, val)| Self::Memory {
                address,
                comparison: cmp,
                value: val,
            }),
            Self::MemoryX {
                address,
                comparison,
                value,
                mask,
            } => invert_cmp_amount(comparison, value).map(|(cmp, val)| Self::MemoryX {
                address,
                comparison: cmp,
                value: val,
                mask,
            }),
            Self::VarCmp {
                var,
                comparison,
                value,
                mask,
            } => invert_cmp_amount(comparison, value).map(|(cmp, val)| Self::VarCmp {
                var,
                comparison: cmp,
                value: val,
                mask,
            }),
            Self::Accumulate {
                player,
                comparison,
                amount,
                resource,
            } => invert_cmp_amount(comparison, amount).map(|(cmp, amt)| Self::Accumulate {
                player,
                comparison: cmp,
                amount: amt,
                resource,
            }),
            Self::Command {
                player,
                comparison,
                amount,
                unit,
            } => invert_cmp_amount(comparison, amount).map(|(cmp, amt)| Self::Command {
                player,
                comparison: cmp,
                amount: amt,
                unit,
            }),
            Self::Bring {
                player,
                comparison,
                amount,
                unit,
                location,
            } => invert_cmp_amount(comparison, amount).map(|(cmp, amt)| Self::Bring {
                player,
                comparison: cmp,
                amount: amt,
                unit,
                location,
            }),
            Self::ElapsedTime {
                comparison,
                seconds,
            } => invert_cmp_amount(comparison, seconds).map(|(cmp, s)| Self::ElapsedTime {
                comparison: cmp,
                seconds: s,
            }),
            Self::Score {
                player,
                score_type,
                comparison,
                amount,
            } => invert_cmp_amount(comparison, amount).map(|(cmp, amt)| Self::Score {
                player,
                score_type,
                comparison: cmp,
                amount: amt,
            }),
            Self::DeathsDynamic { .. }
            | Self::MemoryDynamic { .. }
            | Self::ExprCmp { .. }
            | Self::IsUserCp => None,
        }
    }

    pub fn from_cmp(lhs: HirExpr, op: CmpOp, rhs: HirExpr) -> Self {
        let lhs = lhs.fold_constants();
        let rhs = rhs.fold_constants();

        // 1. Both are constants
        if let (HirExpr::Const(lc), HirExpr::Const(rc)) = (&lhs, &rhs) {
            let lu = *lc as u32;
            let ru = *rc as u32;
            let holds = match op {
                CmpOp::Eq => lu == ru,
                CmpOp::Ne => lu != ru,
                CmpOp::Lt => lu < ru,
                CmpOp::Le => lu <= ru,
                CmpOp::Gt => lu > ru,
                CmpOp::Ge => lu >= ru,
            };
            return if holds { Self::Always } else { Self::Never };
        }

        // 2. Normalize so constant is on rhs if op is commutative or invertible
        match (&lhs, op, &rhs) {
            (HirExpr::Const(c), CmpOp::Eq, other) => {
                return Self::from_cmp(other.clone(), CmpOp::Eq, HirExpr::Const(*c));
            }
            (HirExpr::Const(c), CmpOp::Ne, other) => {
                return Self::from_cmp(other.clone(), CmpOp::Ne, HirExpr::Const(*c));
            }
            (HirExpr::Const(c), CmpOp::Lt, other) => {
                return Self::from_cmp(other.clone(), CmpOp::Gt, HirExpr::Const(*c));
            }
            (HirExpr::Const(c), CmpOp::Le, other) => {
                return Self::from_cmp(other.clone(), CmpOp::Ge, HirExpr::Const(*c));
            }
            (HirExpr::Const(c), CmpOp::Gt, other) => {
                return Self::from_cmp(other.clone(), CmpOp::Lt, HirExpr::Const(*c));
            }
            (HirExpr::Const(c), CmpOp::Ge, other) => {
                return Self::from_cmp(other.clone(), CmpOp::Le, HirExpr::Const(*c));
            }
            _ => {}
        }

        // 3. RHS is constant
        if let HirExpr::Const(rc) = rhs {
            let ru = rc as u32;
            match op {
                CmpOp::Lt => {
                    if ru == 0 {
                        return Self::Never;
                    }
                    return Self::from_cmp(lhs, CmpOp::Le, HirExpr::Const((ru - 1) as i32));
                }
                CmpOp::Le => {
                    if ru == u32::MAX {
                        return Self::Always;
                    }
                    if let HirExpr::Var(v) = lhs {
                        return Self::VarCmp {
                            var: v,
                            comparison: Comparison::AtMost,
                            value: ru,
                            mask: None,
                        };
                    }
                    return Self::ExprCmp {
                        lhs,
                        comparison: Comparison::AtMost,
                        rhs: HirExpr::Const(rc),
                    };
                }
                CmpOp::Gt => {
                    if ru == u32::MAX {
                        return Self::Never;
                    }
                    return Self::from_cmp(lhs, CmpOp::Ge, HirExpr::Const((ru + 1) as i32));
                }
                CmpOp::Ge => {
                    if ru == 0 {
                        return Self::Always;
                    }
                    if let HirExpr::Var(v) = lhs {
                        return Self::VarCmp {
                            var: v,
                            comparison: Comparison::AtLeast,
                            value: ru,
                            mask: None,
                        };
                    }
                    return Self::ExprCmp {
                        lhs,
                        comparison: Comparison::AtLeast,
                        rhs: HirExpr::Const(rc),
                    };
                }
                CmpOp::Eq => {
                    if let HirExpr::Var(v) = lhs {
                        return Self::VarCmp {
                            var: v,
                            comparison: Comparison::Exactly,
                            value: ru,
                            mask: None,
                        };
                    }
                    return Self::ExprCmp {
                        lhs,
                        comparison: Comparison::Exactly,
                        rhs: HirExpr::Const(rc),
                    };
                }
                CmpOp::Ne => {
                    if ru == 0 {
                        return Self::from_cmp(lhs, CmpOp::Ge, HirExpr::Const(1));
                    }
                    if ru == u32::MAX {
                        return Self::from_cmp(lhs, CmpOp::Le, HirExpr::Const(-2));
                        // 0xFFFFFFFE
                    }
                    let diff = HirExpr::BinOp {
                        op: HirBinOp::Sub,
                        lhs: Box::new(lhs),
                        rhs: Box::new(HirExpr::Const(rc)),
                    };
                    return Self::from_cmp(diff, CmpOp::Ge, HirExpr::Const(1));
                }
            }
        }

        // 4. Both sides are dynamic expressions
        match op {
            CmpOp::Eq => Self::ExprCmp {
                lhs,
                comparison: Comparison::Exactly,
                rhs,
            },
            CmpOp::Ne => {
                let diff = HirExpr::BinOp {
                    op: HirBinOp::Sub,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                };
                Self::from_cmp(diff, CmpOp::Ge, HirExpr::Const(1))
            }
            CmpOp::Ge => Self::ExprCmp {
                lhs,
                comparison: Comparison::AtLeast,
                rhs,
            },
            CmpOp::Le => Self::ExprCmp {
                lhs,
                comparison: Comparison::AtMost,
                rhs,
            },
            CmpOp::Gt => {
                // a > b -> (a -sat b) >= 1
                let sat_diff = HirExpr::BinOp {
                    op: HirBinOp::SubSat,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                };
                Self::from_cmp(sat_diff, CmpOp::Ge, HirExpr::Const(1))
            }
            CmpOp::Lt => {
                // a < b -> (b -sat a) >= 1
                let sat_diff = HirExpr::BinOp {
                    op: HirBinOp::SubSat,
                    lhs: Box::new(rhs),
                    rhs: Box::new(lhs),
                };
                Self::from_cmp(sat_diff, CmpOp::Ge, HirExpr::Const(1))
            }
        }
    }
}

impl fmt::Display for StockCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Always => write!(f, "Always()"),
            Self::Never => write!(f, "Never()"),
            Self::Deaths {
                player,
                comparison,
                amount,
                unit,
            } => write!(f, "Deaths({player}, {comparison}, {amount}, unit={unit})"),
            Self::DeathsX {
                player,
                comparison,
                amount,
                unit,
                mask,
            } => write!(
                f,
                "DeathsX({player}, {comparison}, {amount}, unit={unit}, mask={mask:#010X})"
            ),
            Self::DeathsDynamic {
                player,
                comparison,
                amount,
                unit,
                mask: None,
            } => write!(
                f,
                "DeathsDynamic({player}, {comparison}, {amount}, unit={unit})"
            ),
            Self::DeathsDynamic {
                player,
                comparison,
                amount,
                unit,
                mask: Some(mask),
            } => write!(
                f,
                "DeathsDynamic({player}, {comparison}, {amount}, unit={unit}, mask={mask:#010X})"
            ),
            Self::Memory {
                address,
                comparison,
                value,
            } => write!(f, "Memory({address:#010X}, {comparison}, {value})"),
            Self::MemoryX {
                address,
                comparison,
                value,
                mask,
            } => write!(
                f,
                "MemoryX({address:#010X}, {comparison}, {value}, mask={mask:#010X})"
            ),
            Self::MemoryDynamic {
                address,
                comparison,
                value,
                mask: None,
            } => write!(f, "MemoryDynamic({address:#010X}, {comparison}, {value})"),
            Self::MemoryDynamic {
                address,
                comparison,
                value,
                mask: Some(mask),
            } => write!(
                f,
                "MemoryDynamic({address:#010X}, {comparison}, {value}, mask={mask:#010X})"
            ),
            Self::VarCmp {
                var,
                comparison,
                value,
                mask: None,
            } => write!(f, "VarCmp(@v{}, {comparison}, {value})", var.0),
            Self::VarCmp {
                var,
                comparison,
                value,
                mask: Some(mask),
            } => write!(
                f,
                "VarCmp(@v{}, {comparison}, {value}, mask={mask:#010X})",
                var.0
            ),
            Self::ExprCmp {
                lhs,
                comparison,
                rhs,
            } => write!(f, "ExprCmp({lhs}, {comparison}, {rhs})"),
            Self::Accumulate {
                player,
                comparison,
                amount,
                resource,
            } => write!(
                f,
                "Accumulate({player}, {comparison}, {amount}, {resource})"
            ),
            Self::Command {
                player,
                comparison,
                amount,
                unit,
            } => write!(f, "Command({player}, {comparison}, {amount}, unit={unit})"),
            Self::Bring {
                player,
                comparison,
                amount,
                unit,
                location,
            } => write!(
                f,
                "Bring({player}, {comparison}, {amount}, unit={unit}, loc={location})"
            ),
            Self::Switch { switch, state } => write!(f, "Switch({switch}, {state:?})"),
            Self::ElapsedTime {
                comparison,
                seconds,
            } => write!(f, "ElapsedTime({comparison}, {seconds}s)"),
            Self::Score {
                player,
                score_type,
                comparison,
                amount,
            } => write!(
                f,
                "Score({player}, type={score_type}, {comparison}, {amount})"
            ),
            Self::IsUserCp => write!(f, "IsUserCP()"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FormatPart {
    Text(String),
    Decimal(HirExpr),
    Hex(HirExpr),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PrintTarget {
    DisplayText,
    ScreenLine(u8),
    MapString(u32),
    Memory(HirExpr),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StockAction {
    SetDeaths {
        player: PlayerId,
        modifier: Modifier,
        value: HirValue,
        unit: u16,
    },
    SetDeathsX {
        player: PlayerId,
        modifier: Modifier,
        value: HirValue,
        unit: u16,
        mask: u32,
    },
    SetDeathsDynamic {
        player: HirExpr,
        modifier: Modifier,
        value: HirExpr,
        unit: u16,
        mask: Option<u32>,
    },
    SetMemory {
        address: u32,
        modifier: Modifier,
        value: HirValue,
    },
    SetMemoryX {
        address: u32,
        modifier: Modifier,
        value: HirValue,
        mask: u32,
    },
    SetMemoryDynamic {
        address: u32,
        modifier: Modifier,
        value: HirExpr,
        mask: Option<u32>,
    },
    SetVarConst {
        var: VarId,
        modifier: Modifier,
        value: i32,
        mask: Option<u32>,
    },
    SetField {
        target: FieldRef,
        modifier: Modifier,
        value: HirValue,
    },
    SetFieldDynamic {
        target: FieldRef,
        modifier: Modifier,
        value: HirExpr,
        mask: Option<u32>,
    },
    SetResources {
        player: PlayerId,
        modifier: Modifier,
        amount: u32,
        resource: ResourceType,
    },
    DisplayText {
        text: String,
        always_display: bool,
    },
    DisplayTextId {
        string_id: u32,
        always_display: bool,
    },
    FormatPrint {
        target: PrintTarget,
        parts: Vec<FormatPart>,
        always_display: bool,
    },
    PrintAt {
        line: u8,
        parts: Vec<FormatPart>,
    },
    CreateUnit {
        count: u8,
        unit: u16,
        location: u32,
        player: PlayerId,
    },
    KillUnitAt {
        count: u8,
        unit: u16,
        location: u32,
        player: PlayerId,
    },
    MinimapPing {
        location: u32,
    },
    SetScore {
        player: PlayerId,
        modifier: Modifier,
        amount: u32,
        score_type: u8,
    },
    SetSwitch {
        switch: u8,
        action: SwitchAction,
    },
    MoveLocation {
        location: u32,
        unit: u16,
        player: PlayerId,
        dest_location: u32,
    },
    RemoveUnitAt {
        count: u8,
        unit: u16,
        location: u32,
        player: PlayerId,
    },
    KillUnit {
        unit: u16,
        player: PlayerId,
    },
    MoveUnit {
        count: u8,
        unit: u16,
        player: PlayerId,
        start_location: u32,
        dest_location: u32,
    },
    GiveUnits {
        count: u8,
        unit: u16,
        player: PlayerId,
        location: u32,
        new_player: PlayerId,
    },
    Order {
        unit: u16,
        player: PlayerId,
        start_location: u32,
        order_type: u8,
        dest_location: u32,
    },
    PlayWAV {
        path: String,
    },
    Comment {
        text: String,
    },
    PreserveTrigger,
    Victory,
    Defeat,
}

impl fmt::Display for StockAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetDeaths {
                player,
                modifier,
                value,
                unit,
            } => write!(f, "SetDeaths({player}, {modifier}, {value}, unit={unit})"),
            Self::SetDeathsX {
                player,
                modifier,
                value,
                unit,
                mask,
            } => write!(
                f,
                "SetDeathsX({player}, {modifier}, {value}, unit={unit}, mask={mask:#010X})"
            ),
            Self::SetDeathsDynamic {
                player,
                modifier,
                value,
                unit,
                mask: None,
            } => write!(
                f,
                "SetDeathsDynamic({player}, {modifier}, {value}, unit={unit})"
            ),
            Self::SetDeathsDynamic {
                player,
                modifier,
                value,
                unit,
                mask: Some(mask),
            } => write!(
                f,
                "SetDeathsDynamic({player}, {modifier}, {value}, unit={unit}, mask={mask:#010X})"
            ),
            Self::SetMemory {
                address,
                modifier,
                value,
            } => write!(f, "SetMemory({address:#010X}, {modifier}, {value})"),
            Self::SetMemoryX {
                address,
                modifier,
                value,
                mask,
            } => write!(
                f,
                "SetMemoryX({address:#010X}, {modifier}, {value}, mask={mask:#010X})"
            ),
            Self::SetMemoryDynamic {
                address,
                modifier,
                value,
                mask: None,
            } => write!(
                f,
                "SetMemoryDynamic({address:#010X}, {modifier}, {value})"
            ),
            Self::SetMemoryDynamic {
                address,
                modifier,
                value,
                mask: Some(mask),
            } => write!(
                f,
                "SetMemoryDynamic({address:#010X}, {modifier}, {value}, mask={mask:#010X})"
            ),
            Self::SetVarConst {
                var,
                modifier,
                value,
                mask: None,
            } => write!(f, "SetVarConst(@v{}, {modifier}, {value})", var.0),
            Self::SetVarConst {
                var,
                modifier,
                value,
                mask: Some(mask),
            } => write!(
                f,
                "SetVarConst(@v{}, {modifier}, {value}, mask={mask:#010X})",
                var.0
            ),
            Self::SetField {
                target,
                modifier,
                value,
            } => write!(f, "SetField({target}, {modifier}, {value})"),
            Self::SetFieldDynamic {
                target,
                modifier,
                value,
                mask: None,
            } => write!(f, "SetFieldDynamic({target}, {modifier}, {value})"),
            Self::SetFieldDynamic {
                target,
                modifier,
                value,
                mask: Some(mask),
            } => write!(
                f,
                "SetFieldDynamic({target}, {modifier}, {value}, mask={mask:#010X})"
            ),
            Self::SetResources {
                player,
                modifier,
                amount,
                resource,
            } => write!(f, "SetResources({player}, {modifier}, {amount}, {resource})"),
            Self::DisplayText {
                text,
                always_display,
            } => {
                if *always_display {
                    write!(f, "DisplayText({text:?})")
                } else {
                    write!(f, "DisplayText({text:?}, always=false)")
                }
            }
            Self::DisplayTextId {
                string_id,
                always_display,
            } => write!(f, "DisplayTextId(#{string_id}, always={always_display})"),
            Self::FormatPrint { target, parts, always_display } => {
                write!(f, "FormatPrint({target:?}, {parts:?}, always={always_display})")
            }
            Self::PrintAt { line, parts } => write!(f, "PrintAt(line={line}, {parts:?})"),
            Self::CreateUnit {
                count,
                unit,
                location,
                player,
            } => write!(f, "CreateUnit({count}, unit={unit}, loc={location}, {player})"),
            Self::KillUnitAt {
                count,
                unit,
                location,
                player,
            } => write!(f, "KillUnitAt({count}, unit={unit}, loc={location}, {player})"),
            Self::MinimapPing { location } => write!(f, "MinimapPing(loc={location})"),
            Self::SetScore { player, modifier, amount, score_type } => write!(f, "SetScore({player}, {modifier}, {amount}, type={score_type})"),
            Self::SetSwitch { switch, action } => write!(f, "SetSwitch({switch}, {action:?})"),
            Self::MoveLocation { location, unit, player, dest_location } => write!(f, "MoveLocation(loc={location}, unit={unit}, {player}, dest={dest_location})"),
            Self::RemoveUnitAt { count, unit, location, player } => write!(f, "RemoveUnitAt({count}, unit={unit}, loc={location}, {player})"),
            Self::KillUnit { unit, player } => write!(f, "KillUnit(unit={unit}, {player})"),
            Self::MoveUnit { count, unit, player, start_location, dest_location } => write!(f, "MoveUnit({count}, unit={unit}, {player}, from={start_location}, to={dest_location})"),
            Self::GiveUnits { count, unit, player, location, new_player } => write!(f, "GiveUnits({count}, unit={unit}, {player}, loc={location}, to={new_player})"),
            Self::Order { unit, player, start_location, order_type, dest_location } => write!(f, "Order(unit={unit}, {player}, from={start_location}, order={order_type}, to={dest_location})"),
            Self::PlayWAV { path } => write!(f, "PlayWAV({path:?})"),
            Self::Comment { text } => write!(f, "Comment({text:?})"),
            Self::PreserveTrigger => write!(f, "PreserveTrigger()"),
            Self::Victory => write!(f, "Victory()"),
            Self::Defeat => write!(f, "Defeat()"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum InlineHint {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct FuncPtrId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HirFuncPtr {
    pub id: FuncPtrId,
    pub name: String,
    pub signature: (usize, usize),
    pub initial_target: Option<FuncId>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReactiveRule {
    Once {
        cond: Vec<StockCondition>,
        body: Vec<HirStmt>,
    },
    When {
        cond: Vec<StockCondition>,
        body: Vec<HirStmt>,
    },
    Until {
        cond: Vec<StockCondition>,
        body: Vec<HirStmt>,
    },
    After {
        delay_ticks: u32,
        body: Vec<HirStmt>,
    },
    Every {
        period_ticks: u32,
        body: Vec<HirStmt>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirStmt {
    Trigger {
        label: Option<String>,
        conditions: Vec<StockCondition>,
        actions: Vec<StockAction>,
        preserved: bool,
    },
    Action(StockAction),
    VarAssignConst {
        var: VarId,
        value: i32,
    },
    VarAddConst {
        var: VarId,
        value: i32,
    },
    VarSubConst {
        var: VarId,
        value: i32,
    },
    VarSetMaskConst {
        var: VarId,
        modifier: Modifier,
        value: i32,
        mask: u32,
    },
    VarAssignExpr {
        var: VarId,
        expr: HirExpr,
    },
    If {
        conditions: Vec<StockCondition>,
        then_body: Vec<HirStmt>,
        else_body: Vec<HirStmt>,
    },
    While {
        conditions: Vec<StockCondition>,
        body: Vec<HirStmt>,
    },
    Break,
    Continue,
    Call {
        callee: FuncId,
        args: Vec<HirExpr>,
        rets: Vec<VarId>,
    },
    ReturnValues(Vec<HirExpr>),
    FuncPtrAssign {
        ptr: FuncPtrId,
        target: FuncId,
    },
    CallIndirect {
        ptr: FuncPtrId,
        args: Vec<HirExpr>,
        rets: Vec<VarId>,
    },
    MemRead {
        dst: VarId,
        epd: HirExpr,
        mask: u32,
        shift: i32,
        initial_val: i32,
    },
    Reactive(ReactiveRule),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirTerminator {
    Jump(BlockId),
    Branch {
        conditions: Vec<StockCondition>,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HirBasicBlock {
    pub id: BlockId,
    pub label: String,
    pub stmts: Vec<HirStmt>,
    pub terminator: HirTerminator,
}

impl HirBasicBlock {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: BlockId(0),
            label: label.into(),
            stmts: Vec::new(),
            terminator: HirTerminator::Return,
        }
    }

    pub fn push_stmt(&mut self, stmt: HirStmt) {
        self.stmts.push(stmt);
    }

    pub fn set_terminator(&mut self, terminator: HirTerminator) {
        self.terminator = terminator;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HirFunction {
    pub id: FuncId,
    pub name: String,
    pub is_main_loop: bool,
    pub params: Vec<VarId>,
    pub return_count: usize,
    pub inline_hint: InlineHint,
    pub blocks: Vec<HirBasicBlock>,
}

impl HirFunction {
    pub fn new(name: impl Into<String>, is_main_loop: bool) -> Self {
        Self {
            id: FuncId(0),
            name: name.into(),
            is_main_loop,
            params: Vec::new(),
            return_count: 0,
            inline_hint: InlineHint::Auto,
            blocks: Vec::new(),
        }
    }

    pub fn new_main_loop(name: impl Into<String>) -> Self {
        Self::new(name, true)
    }

    pub fn with_params(mut self, params: impl IntoIterator<Item = VarId>) -> Self {
        self.params = params.into_iter().collect();
        self
    }

    pub fn with_return_count(mut self, return_count: usize) -> Self {
        self.return_count = return_count;
        self
    }

    pub fn with_inline_hint(mut self, hint: InlineHint) -> Self {
        self.inline_hint = hint;
        self
    }

    pub fn add_block(&mut self, mut block: HirBasicBlock) -> BlockId {
        let id = BlockId(self.blocks.len() as u32);
        block.id = id;
        self.blocks.push(block);
        id
    }

    pub fn block(&self, id: BlockId) -> &HirBasicBlock {
        &self.blocks[id.0 as usize]
    }

    pub fn block_mut(&mut self, id: BlockId) -> &mut HirBasicBlock {
        &mut self.blocks[id.0 as usize]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HirByteBuf {
    pub id: ByteBufId,
    pub name: String,
    pub data: Vec<u8>,
}

impl HirByteBuf {
    pub fn new(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            id: ByteBufId(0),
            name: name.into(),
            data,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HirModule {
    pub name: String,
    pub functions: Vec<HirFunction>,
    pub main_loop: Option<FuncId>,
    pub byte_bufs: Vec<HirByteBuf>,
    pub vars: Vec<HirVar>,
    pub func_ptrs: Vec<HirFuncPtr>,
}

impl HirModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: Vec::new(),
            main_loop: None,
            byte_bufs: Vec::new(),
            vars: Vec::new(),
            func_ptrs: Vec::new(),
        }
    }

    pub fn add_var(&mut self, name: impl Into<String>, init_val: i32) -> VarId {
        let id = VarId(self.vars.len() as u32);
        let mut var = HirVar::new(name, init_val);
        var.id = id;
        self.vars.push(var);
        id
    }

    pub fn var(&self, id: VarId) -> &HirVar {
        &self.vars[id.0 as usize]
    }

    pub fn add_byte_buf(&mut self, mut buf: HirByteBuf) -> ByteBufId {
        let id = ByteBufId(self.byte_bufs.len() as u32);
        buf.id = id;
        self.byte_bufs.push(buf);
        id
    }

    pub fn add_function(&mut self, mut func: HirFunction) -> FuncId {
        let id = FuncId(self.functions.len() as u32);
        func.id = id;
        if func.is_main_loop {
            self.main_loop = Some(id);
        }
        self.functions.push(func);
        id
    }

    pub fn add_func_ptr(
        &mut self,
        name: impl Into<String>,
        signature: (usize, usize),
        initial_target: Option<FuncId>,
    ) -> FuncPtrId {
        let id = FuncPtrId(self.func_ptrs.len() as u32);
        self.func_ptrs.push(HirFuncPtr {
            id,
            name: name.into(),
            signature,
            initial_target,
        });
        id
    }

    pub fn func_ptr(&self, id: FuncPtrId) -> &HirFuncPtr {
        &self.func_ptrs[id.0 as usize]
    }

    pub fn functions(&self) -> &[HirFunction] {
        &self.functions
    }

    pub fn functions_mut(&mut self) -> &mut [HirFunction] {
        &mut self.functions
    }

    pub fn variables(&self) -> &[HirVar] {
        &self.vars
    }

    pub fn lower_reactive_rules(&mut self) -> crate::error::Result<()> {
        lower_reactive_rules(self)
    }
}
