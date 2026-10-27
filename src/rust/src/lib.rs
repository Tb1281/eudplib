mod allocator;
mod epscript;
mod eudkit_py;
mod eudobj;
pub(crate) mod localize;
mod mpqapi;
mod types;

use pyo3::prelude::*;

#[pymodule]
mod _rust {
    #[pymodule_export]
    use crate::eudkit_py::{
        PyAction, PyCompilationReport, PyCompilationSession, PyCondition, PyExpr, PyMapString,
    };
    #[pymodule_export]
    use crate::eudkit_py::eudkit_mod;
    #[pymodule_export]
    use crate::allocator::allocator_mod;
    #[pymodule_export]
    use crate::epscript::epscript_mod;
    #[pymodule_export]
    use crate::eudobj::eudobj_mod;
    #[pymodule_export]
    use crate::localize::register_translator;
    #[pymodule_export]
    use crate::mpqapi::mpqapi_mod;
}

