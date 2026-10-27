pub mod allocator;
pub mod chk;
pub mod config;
pub mod cp_string;
pub mod error;
pub mod hir;
pub mod injector;
pub mod inlining;
pub mod lir;
pub mod map_context;
pub mod obscurate;
pub mod session;

pub use allocator::{allocate_lir_module, allocate_lir_module_with_seed, stack_objects, AllocatedPayload};
pub use chk::Chk;
pub use config::ProjectConfig;
pub use cp_string::{
    bytes_to_dwords, emit_cpstring_residue_stream, emit_cpstring_residue_stream_exprs,
    emit_cpstring_write_block, emit_format_parts_actions,
};
pub use error::{EudkitError, Result};
pub use hir::{CmpOp, FormatPart, HirBuilder, HirModule, HirVar, PrintTarget, StockCondition, VarId};
pub use injector::{inject_payload_into_chk, CompilationReport};
pub use lir::{FieldRef, LirModule};
pub use map_context::{MapContext, MapString};
pub use session::{CompilationConfig, CompilationSession};
