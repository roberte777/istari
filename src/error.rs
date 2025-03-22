use thiserror::Error;

/// Error types for Istari
#[derive(Error, Debug)]
pub enum IstariError {
    #[error("Duplicate command key '{0}' in menu '{1}'")]
    DuplicateCommand(String, String),
    
    #[error("Reserved command key '{0}' in menu '{1}'")]
    ReservedCommand(String, String),
}

/// Reserved command keys that cannot be used in menus
pub const RESERVED_KEYS: [&str; 2] = ["q", "b"]; 