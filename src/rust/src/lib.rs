mod eudkit_py;

use pyo3::prelude::*;

#[pymodule]
mod _rust {
    #[pymodule_export]
    use crate::eudkit_py::eudkit_mod;
    #[pymodule_export]
    use crate::eudkit_py::{
        PyAction, PyCompilationReport, PyCompilationSession, PyCondition, PyExpr, PyMapString,
    };
}
