//! Runtime lowering of formatted text and mutable string buffers.
//!
//! `FormatPrint`/`PrintAt` used to be lowered with compile-time constant
//! substitution, which silently printed `0` for every runtime variable.  This
//! pass runs before CFG expansion and rewrites those actions into primitive
//! HIR statements: byte writes into a particular `STRx` buffer, decimal and
//! hexadecimal digit extraction, and a final `DisplayTextId`.
//!
//! Doing this at the HIR level lets the existing dynamic-write lowering
//! (`SetMemoryDynamic`/`SetDeathsDynamic`) handle the VProc patching.

use crate::error::{EudkitError, Result};
use crate::hir::{
    epd, Comparison, FormatPart, HirBinOp, HirExpr, HirModule, HirStmt, Modifier, PrintTarget,
    StockAction, StockCondition, StringBufferId, VarId,
};
use crate::map_context::MapContext;

/// Capacity reserved for the shared scratch buffer used by `f_eprintln` and
/// other formatted `DisplayText` actions.
const FORMAT_SCRATCH_CAPACITY: u32 = 512;

/// The address a formatter writes into.
#[derive(Debug, Clone)]
enum FormatBase {
    /// A concrete byte address in the loaded `STRx` section.
    Address(u32),
    /// An already-computed EPD expression, used by `PrintTarget::Memory`.
    Epd(HirExpr),
}

#[derive(Debug, Clone)]
struct FormatScratch {
    string_id: u32,
    address: u32,
}

#[derive(Debug, Clone)]
struct WriterState {
    base: FormatBase,
    /// Byte offset while every write so far is known statically.
    static_pos: Option<u32>,
    /// Byte offset once a conditional/dynamic write makes the position runtime.
    pos_var: Option<VarId>,
}

impl WriterState {
    fn new(base: FormatBase) -> Self {
        Self {
            base,
            static_pos: Some(0),
            pos_var: None,
        }
    }
}

struct FormatExpander {
    hir: HirModule,
    scratch: FormatScratch,
    map_strings: std::collections::HashMap<u32, u32>,
    next_var: u32,
}

impl FormatExpander {
    fn new(
        hir: HirModule,
        scratch: FormatScratch,
        map_strings: std::collections::HashMap<u32, u32>,
    ) -> Self {
        Self {
            hir,
            scratch,
            map_strings,
            next_var: 0,
        }
    }

    fn new_var(&mut self, prefix: &str) -> VarId {
        let name = format!("__fmt_{prefix}_{}", self.next_var);
        self.next_var += 1;
        self.hir.add_var(name, 0)
    }

    fn buffer(&self, id: StringBufferId) -> &crate::hir::HirStringBuffer {
        self.hir.string_buffer(id)
    }

    fn rewrite_module(mut self) -> HirModule {
        let mut functions = std::mem::take(&mut self.hir.functions);
        for func in &mut functions {
            for block in &mut func.blocks {
                let stmts = std::mem::take(&mut block.stmts);
                block.stmts = self.rewrite_stmts(stmts);
            }
        }
        self.hir.functions = functions;
        self.hir
    }

    fn rewrite_stmts(&mut self, stmts: Vec<HirStmt>) -> Vec<HirStmt> {
        let mut out = Vec::with_capacity(stmts.len());
        for stmt in stmts {
            match stmt {
                HirStmt::Action(action) => self.rewrite_action(action, &mut out),
                HirStmt::If {
                    conditions,
                    then_body,
                    else_body,
                } => {
                    let then_body = self.rewrite_stmts(then_body);
                    let else_body = self.rewrite_stmts(else_body);
                    out.push(HirStmt::If {
                        conditions,
                        then_body,
                        else_body,
                    });
                }
                HirStmt::While { conditions, body } => {
                    let body = self.rewrite_stmts(body);
                    out.push(HirStmt::While { conditions, body });
                }
                HirStmt::Trigger {
                    label,
                    conditions,
                    actions,
                    preserved,
                } => {
                    // Formatting inside a raw trigger action list would change
                    // when the generated control flow runs.  Reactive-rule
                    // lowering already turns ordinary text calls into
                    // statements, so leave raw triggers untouched.
                    out.push(HirStmt::Trigger {
                        label,
                        conditions,
                        actions,
                        preserved,
                    });
                }
                other => out.push(other),
            }
        }
        out
    }

    fn rewrite_action(&mut self, action: StockAction, out: &mut Vec<HirStmt>) {
        match action {
            StockAction::DisplayStringBuffer {
                buffer,
                always_display,
            } => {
                let string_id = self.buffer(buffer).string_id;
                out.push(HirStmt::Action(StockAction::DisplayTextId {
                    string_id,
                    always_display,
                }));
            }
            StockAction::FormatStringBuffer {
                buffer,
                parts,
                display,
                always_display,
            } => {
                let buf = self.buffer(buffer).clone();
                let mut writer = WriterState::new(FormatBase::Address(buf.address));
                self.emit_parts(&mut writer, &parts, out);
                self.write_terminator(&mut writer, out);
                if display {
                    out.push(HirStmt::Action(StockAction::DisplayTextId {
                        string_id: buf.string_id,
                        always_display,
                    }));
                }
            }
            StockAction::FormatPrint {
                target,
                parts,
                always_display,
            } => match target {
                PrintTarget::DisplayText => {
                    let mut writer = WriterState::new(FormatBase::Address(self.scratch.address));
                    self.emit_parts(&mut writer, &parts, out);
                    self.write_terminator(&mut writer, out);
                    out.push(HirStmt::Action(StockAction::DisplayTextId {
                        string_id: self.scratch.string_id,
                        always_display,
                    }));
                }
                PrintTarget::ScreenLine(line) => {
                    let addr = 0x640B60u32.wrapping_add(218 * line as u32);
                    let mut writer = WriterState::new(FormatBase::Address(addr));
                    self.emit_parts(&mut writer, &parts, out);
                    self.write_terminator(&mut writer, out);
                }
                PrintTarget::MapString(string_id) => {
                    // `MapString` IDs were registered before this pass; the
                    // resolved address is stable because no further strings
                    // are inserted during emission.
                    let address = self.map_string_address(string_id);
                    let mut writer = WriterState::new(FormatBase::Address(address));
                    self.emit_parts(&mut writer, &parts, out);
                    self.write_terminator(&mut writer, out);
                }
                PrintTarget::Memory(epd_expr) => {
                    let mut writer = WriterState::new(FormatBase::Epd(epd_expr));
                    self.emit_parts(&mut writer, &parts, out);
                    self.write_terminator(&mut writer, out);
                }
            },
            StockAction::PrintAt { line, parts } => {
                let addr = 0x640B60u32.wrapping_add(218 * line as u32);
                let mut writer = WriterState::new(FormatBase::Address(addr));
                self.emit_parts(&mut writer, &parts, out);
                self.write_terminator(&mut writer, out);
            }
            other => out.push(HirStmt::Action(other)),
        }
    }

    /// Address of an existing map string, resolved after all string-table
    /// insertion has finished.
    fn map_string_address(&self, string_id: u32) -> u32 {
        self.map_strings
            .get(&string_id)
            .copied()
            .unwrap_or(crate::hir::STRX_BASE_ADDR)
    }

    fn emit_parts(
        &mut self,
        writer: &mut WriterState,
        parts: &[FormatPart],
        out: &mut Vec<HirStmt>,
    ) {
        for part in parts {
            match part {
                FormatPart::Text(text) => {
                    let bytes = text.as_bytes().to_vec();
                    self.push_text(writer, &bytes, out);
                }
                FormatPart::Decimal(expr) => self.emit_decimal(writer, expr, out),
                FormatPart::Hex(expr) => self.emit_hex(writer, expr, out),
            }
        }
    }

    fn write_terminator(&mut self, writer: &mut WriterState, out: &mut Vec<HirStmt>) {
        self.push_byte(writer, HirExpr::Const(0), out);
    }

    fn push_text(&mut self, writer: &mut WriterState, bytes: &[u8], out: &mut Vec<HirStmt>) {
        for &b in bytes {
            self.push_byte(writer, HirExpr::Const(b as i32), out);
        }
    }

    fn ensure_pos_var(&mut self, writer: &mut WriterState, out: &mut Vec<HirStmt>) -> VarId {
        if let Some(var) = writer.pos_var {
            return var;
        }
        let var = self.new_var("pos");
        let initial = writer.static_pos.unwrap_or(0);
        out.push(HirStmt::VarAssignConst {
            var,
            value: initial as i32,
        });
        writer.pos_var = Some(var);
        writer.static_pos = None;
        var
    }

    fn push_byte(&mut self, writer: &mut WriterState, byte: HirExpr, out: &mut Vec<HirStmt>) {
        if let Some(pos) = writer.static_pos {
            let shift = (pos % 4) * 8;
            let value = if shift == 0 {
                byte
            } else {
                mul_pow2(byte, shift)
            };
            let mask = 0xFFu32 << shift;
            match &writer.base {
                FormatBase::Address(address) => {
                    let word = address.wrapping_add(pos & !3);
                    out.push(HirStmt::Action(StockAction::SetMemoryDynamic {
                        address: word,
                        modifier: Modifier::SetTo,
                        value,
                        mask: Some(mask),
                    }));
                }
                FormatBase::Epd(base) => {
                    let player = add(base.clone(), HirExpr::Const((pos / 4) as i32));
                    out.push(HirStmt::Action(StockAction::SetDeathsDynamic {
                        player,
                        modifier: Modifier::SetTo,
                        value,
                        unit: 0,
                        mask: Some(mask),
                    }));
                }
            }
            writer.static_pos = Some(pos + 1);
        } else {
            self.push_runtime_byte(writer, byte, out);
        }
    }

    /// Builds the statements for one byte and advances the writer position.
    /// Used where the same write must be placed in more than one branch.
    fn push_byte_stmts(&mut self, writer: &mut WriterState, byte: HirExpr) -> Vec<HirStmt> {
        let mut stmts = Vec::new();
        self.push_byte(writer, byte, &mut stmts);
        stmts
    }

    fn push_runtime_byte(
        &mut self,
        writer: &mut WriterState,
        byte: HirExpr,
        out: &mut Vec<HirStmt>,
    ) {
        let pos = self.ensure_pos_var(writer, out);
        let word = self.new_var("word");
        out.push(HirStmt::VarAssignExpr {
            var: word,
            expr: HirExpr::BinOp {
                op: HirBinOp::Shr,
                lhs: Box::new(HirExpr::Var(pos)),
                rhs: Box::new(HirExpr::Const(2)),
            },
        });
        let player = match &writer.base {
            FormatBase::Address(address) => add(HirExpr::Const(epd(*address)), HirExpr::Var(word)),
            FormatBase::Epd(base) => add(base.clone(), HirExpr::Var(word)),
        };

        for align in 0..4u32 {
            let shift = align * 8;
            let value = if shift == 0 {
                byte.clone()
            } else {
                mul_pow2(byte.clone(), shift)
            };
            out.push(HirStmt::If {
                conditions: vec![StockCondition::VarCmp {
                    var: pos,
                    comparison: Comparison::Exactly,
                    value: align,
                    mask: Some(3),
                }],
                then_body: vec![HirStmt::Action(StockAction::SetDeathsDynamic {
                    player: player.clone(),
                    modifier: Modifier::SetTo,
                    value,
                    unit: 0,
                    mask: Some(0xFFu32 << shift),
                })],
                else_body: Vec::new(),
            });
        }
        out.push(HirStmt::VarAddConst { var: pos, value: 1 });
    }

    fn emit_decimal(&mut self, writer: &mut WriterState, expr: &HirExpr, out: &mut Vec<HirStmt>) {
        let folded = expr.fold_constants();
        if let HirExpr::Const(c) = folded {
            let text = (c as u32).to_string();
            self.push_text(writer, text.as_bytes(), out);
            return;
        }

        // Decimal digits are written conditionally, so the output position is
        // no longer a compile-time constant from here on.
        self.ensure_pos_var(writer, out);
        let value = self.new_var("dec_value");
        out.push(HirStmt::VarAssignExpr {
            var: value,
            expr: folded,
        });
        let started = self.new_var("dec_started");
        out.push(HirStmt::VarAssignConst {
            var: started,
            value: 0,
        });

        let mut digits = Vec::new();
        for place in [
            1_000_000_000u32,
            100_000_000,
            10_000_000,
            1_000_000,
            100_000,
            10_000,
            1_000,
            100,
            10,
        ] {
            let digit = self.new_var("dec_digit");
            digits.push(HirStmt::VarAssignConst {
                var: digit,
                value: 0,
            });
            digits.push(HirStmt::While {
                conditions: vec![StockCondition::VarCmp {
                    var: value,
                    comparison: Comparison::AtLeast,
                    value: place,
                    mask: None,
                }],
                body: vec![
                    HirStmt::VarSubConst {
                        var: value,
                        value: place as i32,
                    },
                    HirStmt::VarAddConst {
                        var: digit,
                        value: 1,
                    },
                ],
            });

            let write_digit =
                self.push_byte_stmts(writer, add(HirExpr::Var(digit), HirExpr::Const(48)));
            let mut non_zero = write_digit.clone();
            non_zero.push(HirStmt::VarAssignConst {
                var: started,
                value: 1,
            });
            let mut already_started = write_digit;
            if already_started.is_empty() {
                already_started.push(HirStmt::Action(StockAction::Comment {
                    text: "unreachable".to_string(),
                }));
            }
            digits.push(HirStmt::If {
                conditions: vec![StockCondition::VarCmp {
                    var: digit,
                    comparison: Comparison::Exactly,
                    value: 0,
                    mask: None,
                }],
                then_body: vec![HirStmt::If {
                    conditions: vec![StockCondition::VarCmp {
                        var: started,
                        comparison: Comparison::Exactly,
                        value: 0,
                        mask: None,
                    }],
                    then_body: Vec::new(),
                    else_body: already_started,
                }],
                else_body: non_zero,
            });
        }

        // Units digit is always emitted.
        let unit = self.new_var("dec_unit");
        digits.push(HirStmt::VarAssignConst {
            var: unit,
            value: 0,
        });
        digits.push(HirStmt::While {
            conditions: vec![StockCondition::VarCmp {
                var: value,
                comparison: Comparison::AtLeast,
                value: 1,
                mask: None,
            }],
            body: vec![
                HirStmt::VarSubConst {
                    var: value,
                    value: 1,
                },
                HirStmt::VarAddConst {
                    var: unit,
                    value: 1,
                },
            ],
        });
        let write_unit = self.push_byte_stmts(writer, add(HirExpr::Var(unit), HirExpr::Const(48)));
        digits.extend(write_unit);

        let zero = self.push_byte_stmts(writer, HirExpr::Const(48));
        out.push(HirStmt::If {
            conditions: vec![StockCondition::VarCmp {
                var: value,
                comparison: Comparison::Exactly,
                value: 0,
                mask: None,
            }],
            then_body: zero,
            else_body: digits,
        });
    }

    fn emit_hex(&mut self, writer: &mut WriterState, expr: &HirExpr, out: &mut Vec<HirStmt>) {
        let folded = expr.fold_constants();
        if let HirExpr::Const(c) = folded {
            let text = format!("{:08X}", c as u32);
            self.push_text(writer, text.as_bytes(), out);
            return;
        }

        // Hexadecimal output has a fixed width, so when the position is still
        // static every digit lands at a known byte offset.
        if writer.static_pos.is_some() {
            let value = self.new_var("hex_value");
            out.push(HirStmt::VarAssignExpr {
                var: value,
                expr: folded,
            });
            for i in 0..8u32 {
                let shift = 28 - i * 4;
                let digit = self.new_var("hex_digit");
                out.push(HirStmt::VarAssignExpr {
                    var: digit,
                    expr: HirExpr::BinOp {
                        op: HirBinOp::BitAnd,
                        lhs: Box::new(HirExpr::BinOp {
                            op: HirBinOp::Shr,
                            lhs: Box::new(HirExpr::Var(value)),
                            rhs: Box::new(HirExpr::Const(shift as i32)),
                        }),
                        rhs: Box::new(HirExpr::Const(15)),
                    },
                });
                let conv = self.new_var("hex_char");
                out.push(HirStmt::VarAssignExpr {
                    var: conv,
                    expr: add(HirExpr::Var(digit), HirExpr::Const(48)),
                });
                out.push(HirStmt::If {
                    conditions: vec![StockCondition::VarCmp {
                        var: digit,
                        comparison: Comparison::AtLeast,
                        value: 10,
                        mask: None,
                    }],
                    then_body: vec![HirStmt::VarAssignExpr {
                        var: conv,
                        expr: add(HirExpr::Var(digit), HirExpr::Const(55)),
                    }],
                    else_body: Vec::new(),
                });
                self.push_byte(writer, HirExpr::Var(conv), out);
            }
            return;
        }

        self.ensure_pos_var(writer, out);
        let value = self.new_var("hex_value");
        out.push(HirStmt::VarAssignExpr {
            var: value,
            expr: folded,
        });
        for i in 0..8u32 {
            let shift = 28 - i * 4;
            let digit = self.new_var("hex_digit");
            out.push(HirStmt::VarAssignExpr {
                var: digit,
                expr: HirExpr::BinOp {
                    op: HirBinOp::BitAnd,
                    lhs: Box::new(HirExpr::BinOp {
                        op: HirBinOp::Shr,
                        lhs: Box::new(HirExpr::Var(value)),
                        rhs: Box::new(HirExpr::Const(shift as i32)),
                    }),
                    rhs: Box::new(HirExpr::Const(15)),
                },
            });
            let digit_char =
                self.push_byte_stmts(writer, add(HirExpr::Var(digit), HirExpr::Const(48)));
            let alpha_char =
                self.push_byte_stmts(writer, add(HirExpr::Var(digit), HirExpr::Const(55)));
            out.push(HirStmt::If {
                conditions: vec![StockCondition::VarCmp {
                    var: digit,
                    comparison: Comparison::AtLeast,
                    value: 10,
                    mask: None,
                }],
                then_body: alpha_char,
                else_body: digit_char,
            });
        }
    }
}

fn add(lhs: HirExpr, rhs: HirExpr) -> HirExpr {
    HirExpr::BinOp {
        op: HirBinOp::Add,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    }
}

/// Multiplies a byte-sized dynamic expression by `2^bits`.
///
/// The expression lowerer implements constant multiplication as one
/// conditional add per set bit of the operand, which is far cheaper than its
/// generic constant-shift lowering (one action per remaining bit position).
fn mul_pow2(lhs: HirExpr, bits: u32) -> HirExpr {
    HirExpr::BinOp {
        op: HirBinOp::Mul,
        lhs: Box::new(lhs),
        rhs: Box::new(HirExpr::Const((1u32 << bits) as i32)),
    }
}

fn buffer_storage(initial_text: &str, capacity: u32) -> Vec<u8> {
    let mut bytes = initial_text.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend(std::iter::repeat(b' ').take(capacity as usize));
    bytes
}

fn module_has_formatting(hir: &HirModule) -> bool {
    fn stmts_have(stmts: &[HirStmt]) -> bool {
        stmts.iter().any(|stmt| match stmt {
            HirStmt::Action(
                StockAction::FormatPrint { .. }
                | StockAction::PrintAt { .. }
                | StockAction::DisplayStringBuffer { .. }
                | StockAction::FormatStringBuffer { .. },
            ) => true,
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => stmts_have(then_body) || stmts_have(else_body),
            HirStmt::While { body, .. } => stmts_have(body),
            HirStmt::Trigger { actions, .. } => actions.iter().any(|action| {
                matches!(
                    action,
                    StockAction::FormatPrint { .. }
                        | StockAction::PrintAt { .. }
                        | StockAction::DisplayStringBuffer { .. }
                        | StockAction::FormatStringBuffer { .. }
                )
            }),
            _ => false,
        })
    }

    !hir.string_buffers.is_empty()
        || hir
            .functions
            .iter()
            .any(|f| f.blocks.iter().any(|block| stmts_have(&block.stmts)))
}

fn resolve_display_texts_in_stmts(stmts: &mut [HirStmt], map_ctx: &mut MapContext) {
    for stmt in stmts {
        match stmt {
            HirStmt::Action(action) => resolve_display_text(action, map_ctx),
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                resolve_display_texts_in_stmts(then_body, map_ctx);
                resolve_display_texts_in_stmts(else_body, map_ctx);
            }
            HirStmt::While { body, .. } => resolve_display_texts_in_stmts(body, map_ctx),
            HirStmt::Trigger { actions, .. } => {
                for action in actions {
                    resolve_display_text(action, map_ctx);
                }
            }
            _ => {}
        }
    }
}

fn resolve_display_text(action: &mut StockAction, map_ctx: &mut MapContext) {
    if let StockAction::DisplayText {
        text,
        always_display,
    } = action
    {
        let string_id = map_ctx.get_or_insert_string(text.as_bytes());
        *action = StockAction::DisplayTextId {
            string_id,
            always_display: *always_display,
        };
    }
}

/// Registers every string used by runtime formatting, resolves the final
/// string addresses, then rewrites formatted actions into primitive HIR.
pub fn expand_format_actions(hir: &mut HirModule, map_ctx: &mut MapContext) -> Result<()> {
    // Resolve static `DisplayText` strings first.  They may reuse empty
    // string-table slots and therefore move data offsets around.
    for func in &mut hir.functions {
        for block in &mut func.blocks {
            resolve_display_texts_in_stmts(&mut block.stmts, map_ctx);
        }
    }

    if !module_has_formatting(hir) {
        return Ok(());
    }

    // Mutable buffers get dedicated storage: initial text, a NUL that ends the
    // visible string, and reserved space that runtime writes may overwrite.
    for buffer in &mut hir.string_buffers {
        let storage = buffer_storage(&buffer.initial_text, buffer.capacity);
        buffer.string_id = map_ctx.add_unique_string(&storage);
    }

    // Format output destined for `DisplayText` uses one shared scratch string.
    let scratch_storage = buffer_storage("", FORMAT_SCRATCH_CAPACITY);
    let scratch_id = map_ctx.add_unique_string(&scratch_storage);

    // No more insertions happen after this point, so addresses are final.
    for buffer in &mut hir.string_buffers {
        buffer.address = map_ctx.string_address(buffer.string_id).ok_or_else(|| {
            EudkitError::Ir(format!(
                "failed to resolve address for string buffer '{}'",
                buffer.name
            ))
        })?;
    }
    let scratch_address = map_ctx
        .string_address(scratch_id)
        .ok_or_else(|| EudkitError::Ir("failed to resolve format scratch buffer".to_string()))?;

    let mut map_strings = std::collections::HashMap::new();
    for id in 1..=map_ctx.string_table().len() as u32 {
        if let Some(address) = map_ctx.string_address(id) {
            map_strings.insert(id, address);
        }
    }

    let scratch = FormatScratch {
        string_id: scratch_id,
        address: scratch_address,
    };
    let module = std::mem::replace(hir, HirModule::new("__format_placeholder"));
    let expander = FormatExpander::new(module, scratch, map_strings);
    *hir = expander.rewrite_module();
    Ok(())
}
