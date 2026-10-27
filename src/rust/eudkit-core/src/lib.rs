pub mod allocator;
pub mod chk;
pub mod config;
pub mod error;
pub mod hir;
pub mod injector;
pub mod lir;
pub mod map_context;
pub mod session;

pub use allocator::{allocate_lir_module, stack_objects, AllocatedPayload};
pub use chk::Chk;
pub use config::ProjectConfig;
pub use error::{EudkitError, Result};
pub use hir::HirModule;
pub use injector::{inject_payload_into_chk, CompilationReport};
pub use lir::{FieldRef, LirModule};
pub use map_context::MapContext;
pub use session::{CompilationConfig, CompilationSession};
