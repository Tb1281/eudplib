use crate::lir::FieldRef;
use std::fmt::{self, Write};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteBufId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub const fn from_player_slot(slot: u8) -> Self {
        match slot {
            0 => Self::Player1,
            1 => Self::Player2,
            2 => Self::Player3,
            3 => Self::Player4,
            4 => Self::Player5,
            5 => Self::Player6,
            6 => Self::Player7,
            7 => Self::Player8,
            8 => Self::Player9,
            9 => Self::Player10,
            10 => Self::Player11,
            11 => Self::Player12,
            13 => Self::CurrentPlayer,
            17 => Self::AllPlayers,
            other => Self::Epd(other as i32),
        }
    }

    pub const fn encode(self) -> i32 {
        match self {
            Self::Player1 => 0,
            Self::Player2 => 1,
            Self::Player3 => 2,
            Self::Player4 => 3,
            Self::Player5 => 4,
            Self::Player6 => 5,
            Self::Player7 => 6,
            Self::Player8 => 7,
            Self::Player9 => 8,
            Self::Player10 => 9,
            Self::Player11 => 10,
            Self::Player12 => 11,
            Self::CurrentPlayer => 13,
            Self::Foes => 14,
            Self::Allies => 15,
            Self::NeutralPlayers => 16,
            Self::AllPlayers => 17,
            Self::Force1 => 18,
            Self::Force2 => 19,
            Self::Force3 => 20,
            Self::Force4 => 21,
            Self::NonAlliedVictoryPlayers => 26,
            Self::Epd(v) => v,
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    AtLeast,
    AtMost,
    Exactly,
}

impl Comparison {
    pub const fn encode(self) -> u8 {
        match self {
            Self::AtLeast => 0,
            Self::AtMost => 1,
            Self::Exactly => 10,
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchState {
    Set,
    Cleared,
}

impl SwitchState {
    pub const fn encode(self) -> u8 {
        match self {
            Self::Set => 2,
            Self::Cleared => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    SetTo,
    Add,
    Subtract,
}

impl Modifier {
    pub const fn encode(self) -> u8 {
        match self {
            Self::SetTo => 7,
            Self::Add => 8,
            Self::Subtract => 9,
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Ore,
    Gas,
    OreAndGas,
}

impl ResourceType {
    pub const fn encode(self) -> u8 {
        match self {
            Self::Ore => 0,
            Self::Gas => 1,
            Self::OreAndGas => 2,
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirValue {
    Const(i32),
    ByteBufAddress(ByteBufId, u32),
    ByteBufEpd(ByteBufId, i32),
    FieldAddress(FieldRef),
    FieldEpd(FieldRef),
}

impl fmt::Display for HirValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(c) => write!(f, "{c}"),
            Self::ByteBufAddress(id, off) => write!(f, "addr(@buf{}+{})", id.0, off),
            Self::ByteBufEpd(id, off) => write!(f, "epd(@buf{}+{})", id.0, off),
            Self::FieldAddress(fr) => write!(f, "addr({fr})"),
            Self::FieldEpd(fr) => write!(f, "epd({fr})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            Self::Accumulate {
                player,
                comparison,
                amount,
                resource,
            } => write!(f, "Accumulate({player}, {comparison}, {amount}, {resource})"),
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    SetField {
        target: FieldRef,
        modifier: Modifier,
        value: HirValue,
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
    CreateUnit {
        count: u8,
        unit: u16,
        location: u32,
        player: PlayerId,
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
            Self::SetField {
                target,
                modifier,
                value,
            } => write!(f, "SetField({target}, {modifier}, {value})"),
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
            Self::CreateUnit {
                count,
                unit,
                location,
                player,
            } => write!(f, "CreateUnit({count}, unit={unit}, loc={location}, {player})"),
            Self::Comment { text } => write!(f, "Comment({text:?})"),
            Self::PreserveTrigger => write!(f, "PreserveTrigger()"),
            Self::Victory => write!(f, "Victory()"),
            Self::Defeat => write!(f, "Defeat()"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirStmt {
    Trigger {
        label: Option<String>,
        conditions: Vec<StockCondition>,
        actions: Vec<StockAction>,
        preserved: bool,
    },
    Action(StockAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirTerminator {
    Jump(BlockId),
    Branch {
        conditions: Vec<StockCondition>,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirFunction {
    pub id: FuncId,
    pub name: String,
    pub is_main_loop: bool,
    pub blocks: Vec<HirBasicBlock>,
}

impl HirFunction {
    pub fn new(name: impl Into<String>, is_main_loop: bool) -> Self {
        Self {
            id: FuncId(0),
            name: name.into(),
            is_main_loop,
            blocks: Vec::new(),
        }
    }

    pub fn new_main_loop(name: impl Into<String>) -> Self {
        Self::new(name, true)
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirModule {
    pub name: String,
    pub functions: Vec<HirFunction>,
    pub main_loop: Option<FuncId>,
    pub byte_bufs: Vec<HirByteBuf>,
}

impl HirModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: Vec::new(),
            main_loop: None,
            byte_bufs: Vec::new(),
        }
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

    pub fn dump_hir(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "module @{} {{", self.name);
        for buf in &self.byte_bufs {
            let _ = writeln!(
                out,
                "  bytebuf @{} (id=buf{}, len={}) = {:02X?}",
                buf.name,
                buf.id.0,
                buf.data.len(),
                buf.data
            );
        }
        for func in &self.functions {
            let role = if func.is_main_loop {
                " (main_loop)"
            } else {
                ""
            };
            let _ = writeln!(out, "  fn @{}{} {{", func.name, role);
            for block in &func.blocks {
                let _ = writeln!(out, "    bb{} (@{}):", block.id.0, block.label);
                for stmt in &block.stmts {
                    match stmt {
                        HirStmt::Trigger {
                            label,
                            conditions,
                            actions,
                            preserved,
                        } => {
                            let lbl = label.as_deref().unwrap_or("anon");
                            let _ =
                                writeln!(out, "      trigger @{lbl} (preserved={preserved}) {{");
                            for c in conditions {
                                let _ = writeln!(out, "        cond: {c}");
                            }
                            for a in actions {
                                let _ = writeln!(out, "        act:  {a}");
                            }
                            let _ = writeln!(out, "      }}");
                        }
                        HirStmt::Action(act) => {
                            let _ = writeln!(out, "      act: {act}");
                        }
                    }
                }
                match &block.terminator {
                    HirTerminator::Jump(target) => {
                        let _ = writeln!(out, "      jump -> bb{}", target.0);
                    }
                    HirTerminator::Branch {
                        conditions,
                        then_block,
                        else_block,
                    } => {
                        let cond_strs: Vec<String> =
                            conditions.iter().map(ToString::to_string).collect();
                        let _ = writeln!(
                            out,
                            "      branch [{}] -> then bb{}, else bb{}",
                            cond_strs.join(", "),
                            then_block.0,
                            else_block.0
                        );
                    }
                    HirTerminator::Return => {
                        let _ = writeln!(out, "      return");
                    }
                }
            }
            let _ = writeln!(out, "  }}");
        }
        let _ = writeln!(out, "}}");
        out
    }
}
