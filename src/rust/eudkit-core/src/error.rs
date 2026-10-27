use thiserror::Error;

#[derive(Debug, Error)]
pub enum EudkitError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("MPQ archive error: {0}")]
    Storm(#[from] eudplib_stormlib::error::StormError),

    #[error("invalid CHK section header or length at byte offset {offset}")]
    InvalidChkSection { offset: usize },

    #[error("missing required CHK section: {0}")]
    MissingSection(String),

    #[error("invalid section size for {section}: {size} bytes")]
    InvalidSectionSize { section: String, size: usize },

    #[error("unknown symbol '{name}' of kind {kind}")]
    UnknownSymbol { kind: &'static str, name: String },

    #[error("ambiguous symbol '{name}' of kind {kind}")]
    AmbiguousSymbol { kind: &'static str, name: String },

    #[error("IR validation or lowering error: {0}")]
    Ir(String),

    #[error("allocation / relocation error: {0}")]
    Allocation(String),

    #[error("configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, EudkitError>;
