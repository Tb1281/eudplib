use crate::hir::{HirExpr, VarId};
use crate::lir::types::{
    ExpandedBlock, ExpandedOp, FieldRef, ObjectId, TriggerSlotField, VProcSlotField, ValueExpr,
};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarStorage {
    ByteBuf(ObjectId),
    VProcSlot { vbuf_obj: ObjectId, slot_idx: u32 },
}

impl VarStorage {
    pub fn value_epd(self) -> ValueExpr {
        match self {
            Self::ByteBuf(obj) => {
                ValueExpr::FieldEpd(FieldRef::new(obj, TriggerSlotField::ByteOffset(0)))
            }
            Self::VProcSlot { vbuf_obj, slot_idx } => ValueExpr::FieldEpd(FieldRef::vproc_slot(
                vbuf_obj.0,
                slot_idx,
                VProcSlotField::Value,
            )),
        }
    }

    pub fn value_address(self) -> ValueExpr {
        match self {
            Self::ByteBuf(obj) => ValueExpr::ObjectAddress {
                object: obj,
                byte_offset: 0,
            },
            Self::VProcSlot { vbuf_obj, slot_idx } => ValueExpr::FieldAddress(
                FieldRef::vproc_slot(vbuf_obj.0, slot_idx, VProcSlotField::Value),
            ),
        }
    }

    pub fn vproc_field(self, field: VProcSlotField) -> FieldRef {
        match self {
            Self::VProcSlot { vbuf_obj, slot_idx } => {
                FieldRef::vproc_slot(vbuf_obj.0, slot_idx, field)
            }
            Self::ByteBuf(_) => panic!("variable was not promoted to VProcBuffer"),
        }
    }
}

/// Identifies all variables in expanded blocks that require 72-byte VProc storage slots
/// (self-modifying triggers) because their values are dynamically copied or patched.
/// Variables not in this set are safely lowered to 4-byte ByteBuf memory cells per ADR 0002.
pub fn collect_vproc_vars(blocks: &[ExpandedBlock]) -> HashSet<VarId> {
    let mut set = HashSet::new();
    for blk in blocks {
        for op in &blk.ops {
            match op {
                ExpandedOp::VProcCopyVar { src, .. }
                | ExpandedOp::VProcBitAndVar { src, .. }
                | ExpandedOp::VProcBitOrVar { src, .. }
                | ExpandedOp::VProcBitXorVar { src, .. }
                | ExpandedOp::PatchNextTriggerCond { src, .. }
                | ExpandedOp::PatchNextTriggerAct { src, .. } => {
                    set.insert(*src);
                }
                ExpandedOp::DynamicShl { dst, .. } => {
                    set.insert(*dst);
                }
                ExpandedOp::DynamicShr {
                    pow2_scratch,
                    div_x_scratch,
                    ..
                } => {
                    set.insert(*pow2_scratch);
                    set.insert(*div_x_scratch);
                }
                ExpandedOp::DynamicMul { rhs, .. } => {
                    set.insert(*rhs);
                }
                ExpandedOp::DynamicDivRem {
                    divisor,
                    div_x_scratch,
                    ..
                } => {
                    set.insert(*divisor);
                    set.insert(*div_x_scratch);
                }
                ExpandedOp::WriteFuncReturnSlot {
                    src_var: Some(src), ..
                } => {
                    set.insert(*src);
                }
                ExpandedOp::MemReadSetup { epd, .. } => {
                    if let HirExpr::Var(v) = epd {
                        set.insert(*v);
                    }
                }
                ExpandedOp::LeafAction(_)
                | ExpandedOp::Trigger { .. }
                | ExpandedOp::CallSetup { .. }
                | ExpandedOp::CallIndirectSetup { .. }
                | ExpandedOp::WriteFuncReturnSlot { src_var: None, .. }
                | ExpandedOp::FuncPtrAssignTarget { .. } => {}
            }
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::Modifier;
    use crate::lir::types::ExpandedTerminator;

    #[test]
    fn test_constant_only_variables_do_not_require_vproc() {
        let block = ExpandedBlock {
            id: crate::hir::BlockId(0),
            label: "test".to_string(),
            ops: vec![ExpandedOp::LeafAction(
                crate::hir::StockAction::SetVarConst {
                    var: VarId(1),
                    value: 42,
                    modifier: Modifier::SetTo,
                    mask: None,
                },
            )],
            terminator: ExpandedTerminator::Return,
        };
        let vproc_vars = collect_vproc_vars(&[block]);
        assert!(!vproc_vars.contains(&VarId(1)));
    }

    #[test]
    fn test_dynamic_copied_variables_require_vproc() {
        let block = ExpandedBlock {
            id: crate::hir::BlockId(0),
            label: "test".to_string(),
            ops: vec![ExpandedOp::VProcCopyVar {
                src: VarId(2),
                dst: VarId(3),
                modifier: Modifier::SetTo,
                mask: None,
            }],
            terminator: ExpandedTerminator::Return,
        };
        let vproc_vars = collect_vproc_vars(&[block]);
        assert!(vproc_vars.contains(&VarId(2)));
        assert!(!vproc_vars.contains(&VarId(3)));
    }
}
