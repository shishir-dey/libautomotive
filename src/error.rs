use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AutomotiveError {
    // Physical layer errors
    CanError(String),
    CanFdError(String),

    // Transport layer errors
    IsoTpError(String),

    // Network layer errors
    J1939Error(String),

    // Application layer errors
    UdsError(String),
    ObdError(String),

    // Generic errors
    Timeout,
    BufferOverflow,
    InvalidParameter,
    NotInitialized,
    PortError(String),
}

impl fmt::Display for AutomotiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AutomotiveError::CanError(msg) => write!(f, "CAN error: {}", msg),
            AutomotiveError::CanFdError(msg) => write!(f, "CAN FD error: {}", msg),
            AutomotiveError::IsoTpError(msg) => write!(f, "ISO-TP error: {}", msg),
            AutomotiveError::J1939Error(msg) => write!(f, "J1939 error: {}", msg),
            AutomotiveError::UdsError(msg) => write!(f, "UDS error: {}", msg),
            AutomotiveError::ObdError(msg) => write!(f, "OBD error: {}", msg),
            AutomotiveError::Timeout => write!(f, "Operation timed out"),
            AutomotiveError::BufferOverflow => write!(f, "Buffer overflow"),
            AutomotiveError::InvalidParameter => write!(f, "Invalid parameter"),
            AutomotiveError::NotInitialized => write!(f, "Component not initialized"),
            AutomotiveError::PortError(msg) => write!(f, "Port error: {}", msg),
        }
    }
}

impl Error for AutomotiveError {}

pub type Result<T> = std::result::Result<T, AutomotiveError>;
