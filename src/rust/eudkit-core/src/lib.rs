pub mod allocator;
pub mod canonical;
pub mod chk;
pub mod chk_layout;
pub mod config;
pub mod cp_string;
pub mod dts;
pub mod error;
pub mod hir;
pub mod injector;
pub mod inlining;
pub mod lir;
pub mod map_context;
pub mod obscurate;
pub mod plan;
pub mod session;
pub mod symbols;
pub mod vocabulary;

pub use allocator::{
    allocate_lir_module, allocate_lir_module_with_seed, stack_objects, AllocatedPayload,
};
pub use canonical::{CanonicalSymbols, CanonicalUnit};
pub use chk::Chk;
pub use config::{BuildRequest, ProjectConfig, ResolvedBuild};
pub use cp_string::{
    bytes_to_dwords, emit_cpstring_residue_stream, emit_cpstring_residue_stream_exprs,
    emit_cpstring_write_block, emit_format_parts_actions,
};
pub use error::{EudkitError, Result};
pub use hir::{
    CmpOp, FormatPart, HirBuilder, HirModule, HirVar, PrintTarget, StockCondition, VarId,
};
pub use injector::{inject_payload_into_chk, CompilationReport};
pub use lir::{FieldRef, LirModule};
pub use map_context::{MapData, MapString, MapSymbol};
pub use plan::{fingerprint_bytes, HirPlan, PlanMap, HIR_PLAN_FORMAT, HIR_PLAN_SCHEMA_VERSION};
pub use session::{CompilationConfig, CompilationSession};
pub use vocabulary::{
    frontend_constant, frontend_constants, FrontendConstant, OrderType, ScoreType, VocabEntry,
};
