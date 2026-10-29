//! Single owner of the trigger shape of an expanded LIR block.
//!
//! Every [`ExpandedOp`] contributes a fixed number of triggers to the LIR
//! trigger span of its block, may leave actions pending for coalescing with the
//! next op, and may contain `PatchNextTrigger*` sites. VProc choreography is
//! part of that shape too: a VProc op emits its setup/apply triggers and leaves
//! its mask-restore action pending. [`block_trigger_shape`] simulates that
//! policy once for a block.
//!
//! [`crate::lir::EmissionPlan`] reserves every trigger `ObjectId` from this
//! shape before the Backend writes the first trigger, and
//! [`crate::lir::emit_lir`] consumes the planned per-op spans instead of
//! recomputing trigger offsets. Keeping the policy here prevents the plan and
//! the emitter from drifting into two copies of the same match.

use crate::hir::StockAction;
use crate::lir::types::{ExpandedBlock, ExpandedOp, ExpandedTerminator};
use std::collections::HashMap;

/// Triggers emitted by one `DynamicMul` op.
///
/// One init trigger, two triggers per bit for all 32 bits, two triggers per
/// early-exit check for the first 31 bits, and one end trigger.
pub const DYNAMIC_MUL_TRIGGERS: u32 = 1 + 32 * 2 + 31 * 2 + 1;

/// Triggers emitted by the 32 fill iterations of a dynamic division chain.
pub const DYNAMIC_DIV_FILL_TRIGGERS: u32 = 32 * 5;

/// Triggers emitted by one complete dynamic division chain.
///
/// One init trigger, [`DYNAMIC_DIV_FILL_TRIGGERS`] fill triggers, and 32
/// subtraction-chain triggers.
pub const DYNAMIC_DIV_CHAIN_TRIGGERS: u32 = 1 + DYNAMIC_DIV_FILL_TRIGGERS + 32;

/// Triggers emitted by one `DynamicShr` op before its division chain.
pub const DYNAMIC_SHR_PREFIX_TRIGGERS: u32 = 2 + 1 + 32;

/// Total triggers emitted by one `DynamicShr` op.
pub const DYNAMIC_SHR_TRIGGERS: u32 = DYNAMIC_SHR_PREFIX_TRIGGERS + DYNAMIC_DIV_CHAIN_TRIGGERS;

/// Trigger ids reserved for one [`ExpandedOp`], relative to its block start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpTriggerSpan {
    /// First trigger id (relative to the block's first trigger) written while
    /// lowering this op, including any coalesced flush it causes.
    pub first_trigger: u32,
    /// Triggers written while lowering this op.
    pub trigger_count: u32,
    /// Actions left in the coalescing buffer after this op.
    pub pending_actions_after: usize,
    /// `PatchNextTrigger*` sites owned by this op.
    pub patch_sites: u32,
}

impl OpTriggerSpan {
    /// One past the last trigger written while lowering this op.
    pub fn end_trigger(&self) -> u32 {
        self.first_trigger + self.trigger_count
    }
}

/// Trigger ids reserved for one [`ExpandedTerminator`], relative to its block
/// start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminatorTriggerSpan {
    /// First trigger id (relative to the block's first trigger) written while
    /// lowering the terminator.
    pub first_trigger: u32,
    /// Triggers written while lowering the terminator.
    pub trigger_count: u32,
}

impl TerminatorTriggerSpan {
    /// One past the last trigger written while lowering the terminator.
    pub fn end_trigger(&self) -> u32 {
        self.first_trigger + self.trigger_count
    }
}

/// The complete trigger shape of one expanded LIR block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockTriggerShape {
    /// One span per [`ExpandedOp`], in block order.
    pub ops: Vec<OpTriggerSpan>,
    /// The span owned by the block terminator.
    pub terminator: TerminatorTriggerSpan,
    /// Total triggers reserved for the block.
    pub trigger_count: u32,
    /// Total `PatchNextTrigger*` sites in the block.
    pub patch_sites: u32,
}

impl BlockTriggerShape {
    /// Returns every per-op span in block order.
    pub fn op_spans(&self) -> &[OpTriggerSpan] {
        &self.ops
    }

    /// Returns the span for the op at `index`.
    pub fn op_span(&self, index: usize) -> &OpTriggerSpan {
        &self.ops[index]
    }
}

#[derive(Debug, Default)]
struct ShapeState {
    triggers: u32,
    pending_actions: usize,
    patch_sites: u32,
}

impl ShapeState {
    /// Mirrors the emitter's capacity check before pushing a bulk action group.
    fn ensure_capacity(&mut self, n: usize) {
        if self.pending_actions + n > 64 {
            self.flush_pending();
        }
    }

    fn push_actions(&mut self, n: usize) {
        self.pending_actions += n;
        if self.pending_actions == 64 {
            self.flush_pending();
        }
    }

    fn add_pending(&mut self, n: usize) {
        self.ensure_capacity(n);
        self.push_actions(n);
    }

    fn flush_pending(&mut self) {
        if self.pending_actions > 0 {
            self.triggers += 1;
            self.pending_actions = 0;
        }
    }
}

/// Computes the one canonical trigger shape for an expanded LIR block.
///
/// Both [`crate::lir::EmissionPlan::build`] and [`crate::lir::emit_lir`] consume
/// the returned spans, so this function is the only place that knows how many
/// triggers each [`ExpandedOp`] reserves.
pub fn block_trigger_shape(blk: &ExpandedBlock) -> BlockTriggerShape {
    let mut state = ShapeState::default();
    let mut ops = Vec::with_capacity(blk.ops.len());

    for op in &blk.ops {
        let first_trigger = state.triggers;
        let patch_sites_before = state.patch_sites;
        apply_op(op, &mut state);
        ops.push(OpTriggerSpan {
            first_trigger,
            trigger_count: state.triggers - first_trigger,
            pending_actions_after: state.pending_actions,
            patch_sites: state.patch_sites - patch_sites_before,
        });
    }

    let first_trigger = state.triggers;
    apply_terminator(&blk.terminator, &mut state);
    let terminator = TerminatorTriggerSpan {
        first_trigger,
        trigger_count: state.triggers - first_trigger,
    };

    BlockTriggerShape {
        ops,
        terminator,
        trigger_count: state.triggers,
        patch_sites: state.patch_sites,
    }
}

fn apply_op(op: &ExpandedOp, state: &mut ShapeState) {
    match op {
        ExpandedOp::LeafAction(act) => {
            let n = count_leaf_action_actions(act);
            for _ in 0..n {
                state.add_pending(1);
            }
        }
        ExpandedOp::Trigger { .. } => {
            state.flush_pending();
            state.triggers += 1;
        }
        ExpandedOp::VProcCopyVar { mask, .. } => {
            let n = if mask.is_some() { 4 } else { 3 };
            state.add_pending(n);
            state.flush_pending();
            if mask.is_some() {
                state.add_pending(1);
            }
        }
        ExpandedOp::VProcBitAndVar { .. } => {
            state.add_pending(4);
            state.flush_pending();
            state.triggers += 1;
        }
        ExpandedOp::VProcBitOrVar { .. } => {
            state.add_pending(3);
            state.flush_pending();
            state.triggers += 1;
        }
        ExpandedOp::VProcBitXorVar { .. } => {
            state.add_pending(4);
            state.flush_pending();
            state.triggers += 1;
            state.add_pending(1);
        }
        ExpandedOp::DynamicShl { .. } => {
            state.add_pending(3);
            state.flush_pending();
            state.triggers += 2;
            state.add_pending(1);
        }
        ExpandedOp::DynamicShr { .. } => {
            state.flush_pending();
            state.triggers += DYNAMIC_SHR_TRIGGERS;
        }
        ExpandedOp::DynamicMul { .. } => {
            state.flush_pending();
            state.triggers += DYNAMIC_MUL_TRIGGERS;
        }
        ExpandedOp::DynamicDivRem { .. } => {
            state.flush_pending();
            state.triggers += DYNAMIC_DIV_CHAIN_TRIGGERS;
        }
        ExpandedOp::PatchNextTriggerCond { .. } | ExpandedOp::PatchNextTriggerAct { .. } => {
            state.patch_sites += 1;
            state.add_pending(3);
            state.flush_pending();
        }
        ExpandedOp::CallSetup { rets, .. } | ExpandedOp::CallIndirectSetup { rets, .. } => {
            // Reserve the full return-action group. The plan has always shaped
            // calls from `rets` rather than from the callee's planned VProc
            // slots, so this keeps the reserved span stable.
            let act_count = if rets.is_empty() { 1 } else { rets.len() + 1 };
            state.add_pending(act_count);
        }
        ExpandedOp::WriteFuncReturnSlot {
            src_var, const_val, ..
        } => {
            if const_val.is_some() {
                state.add_pending(1);
            } else if src_var.is_some() {
                state.add_pending(3);
                state.flush_pending();
            }
        }
        ExpandedOp::FuncPtrAssignTarget { .. } => {
            state.add_pending(1);
        }
        ExpandedOp::MemReadSetup {
            epd, initial_val, ..
        } => {
            let acts = if *initial_val != 0 { 3 } else { 2 };
            match epd {
                crate::hir::HirExpr::Const(_) => {
                    state.add_pending(acts + 1);
                }
                crate::hir::HirExpr::Var(_) => {
                    state.add_pending(acts + 3);
                    state.flush_pending();
                }
                _ => unreachable!("lowered MemReadSetup epd must be Const or Var"),
            }
        }
    }
}

fn apply_terminator(terminator: &ExpandedTerminator, state: &mut ShapeState) {
    if matches!(terminator, ExpandedTerminator::Branch { .. }) {
        state.add_pending(1);
        state.flush_pending();
        state.triggers += 1;
    } else {
        state.flush_pending();
        if state.triggers == 0 {
            state.triggers += 1;
        }
    }
}

/// Number of actions a leaf [`StockAction`] lowers to.
///
/// Formatting actions expand into a CPString streaming sequence. The count is
/// computed with the same empty variable-initializer table that the plan has
/// always used, so the plan's reserved span is unchanged.
fn count_leaf_action_actions(act: &StockAction) -> usize {
    match act {
        StockAction::PrintAt { parts, .. } => crate::cp_string::emit_format_parts_actions(
            crate::lir::types::ValueExpr::Const(0),
            parts,
            None,
            false,
            &HashMap::new(),
        )
        .len(),
        StockAction::FormatPrint {
            target,
            parts,
            always_display,
        } => {
            let has_disp = matches!(target, crate::hir::PrintTarget::DisplayText);
            crate::cp_string::emit_format_parts_actions(
                crate::lir::types::ValueExpr::Const(0),
                parts,
                if has_disp { Some(1) } else { None },
                *always_display,
                &HashMap::new(),
            )
            .len()
        }
        _ => 1,
    }
}
