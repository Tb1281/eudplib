pub mod compiler;
pub mod error;

pub use compiler::compile_ts_to_hir;
pub use error::{EudkitTsError, Result};
use eudkit_core::hir::HirModule;
use eudkit_core::map_context::MapContext;
use std::fs;
use std::path::Path;

pub fn compile_ts_file<P: AsRef<Path>>(
    path: P,
    module_name: &str,
    map_context: &MapContext,
) -> Result<HirModule> {
    let source = fs::read_to_string(path.as_ref())?;
    compile_ts_to_hir(&source, module_name, map_context)
}
