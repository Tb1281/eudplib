pub use crate::cp_string::*;
use crate::error::{EudkitError, Result};
use crate::hir::HirModule;
use crate::map_context::MapContext;

pub mod cfg_expansion;
pub mod cp_scheduler;
pub mod emission_plan;
pub mod expr_lowerer;
pub mod read_table;
pub mod storage_selection;
pub mod trigger_emitter;
pub mod types;

pub use cp_scheduler::schedule_cp_cursor;
pub use emission_plan::{BlockPlan, EmissionPlan, PlannedObjects};
pub use expr_lowerer::lower_expr_into;
pub use read_table::ReadTable;
pub use storage_selection::{collect_vproc_vars, VarStorage};
pub use trigger_emitter::emit_lir;
pub use types::*;

pub fn lower_hir_to_lir(hir_in: &HirModule, map_ctx: &mut MapContext) -> Result<LirModule> {
    let mut hir = hir_in.clone();
    hir.lower_reactive_rules()?;
    crate::inlining::inline_functions(&mut hir)?;
    crate::format_expand::expand_format_actions(&mut hir, map_ctx)?;
    let main_fn_id = hir
        .main_loop
        .or_else(|| hir.functions.first().map(|f| f.id))
        .ok_or_else(|| EudkitError::Ir("HIR module has no functions".to_string()))?;
    if hir.functions[main_fn_id.0 as usize].blocks.is_empty() {
        return Err(EudkitError::Ir(format!(
            "function '{}' has no basic blocks",
            hir.functions[main_fn_id.0 as usize].name
        )));
    }

    let mut scratch = types::ScratchAllocator::new();
    let expanded_cfg = cfg_expansion::lower_and_expand_cfg(&mut hir, main_fn_id, &mut scratch)?;
    trigger_emitter::emit_lir(&hir, expanded_cfg, map_ctx)
}
