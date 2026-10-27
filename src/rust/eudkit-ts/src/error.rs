use thiserror::Error;

pub type Result<T> = std::result::Result<T, EudkitTsError>;

#[derive(Debug, Error)]
pub enum EudkitTsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TypeScript parse error: {0}")]
    Parse(String),

    #[error("TypeScript compile error: {0}")]
    Compile(String),

    #[error("Core compiler error: {0}")]
    Core(#[from] eudkit_core::error::EudkitError),
}
