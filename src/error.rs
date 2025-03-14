//! Error types for the automotive protocol stack.
//!
//! This module provides a unified error handling system for all layers of the protocol stack,
//! from physical layer (CAN) up to application layer (UDS, OBD-II).

use std::error::Error;
use std::fmt;
use std::io;

/// Represents all possible errors that can occur in the automotive protocol stack.
///
/// This enum encompasses errors from all layers of the protocol stack, providing
/// specific error variants for each protocol as well as generic error conditions.
#[derive(Debug)]
pub enum AutomotiveError {
    /// Errors related to CAN bus operations
    CanError(String),
    /// Errors specific to CAN-FD operations
    CanFdError(String),

    /// Errors occurring in ISO-TP (ISO 15765-2) protocol
    IsoTpError(String),

    /// Errors specific to J1939 protocol operations
    J1939Error(String),

    /// Errors occurring in UDS (ISO 14229) protocol
    UdsError(String),
    /// Errors specific to OBD-II operations
    ObdError(String),

    /// Errors specific to DoIP operations
    DoIPError(String),
    /// Connection failed in DoIP
    ConnectionFailed,
    /// Failed to send data in DoIP
    SendFailed,
    /// Failed to receive data in DoIP
    ReceiveFailed,

    /// Operation timed out
    Timeout,
    /// Buffer capacity exceeded
    BufferOverflow,
    /// Invalid parameter provided to function
    InvalidParameter,
    /// Component used before initialization
    NotInitialized,
    /// Error related to hardware port operations
    PortError(String),

    /// Invalid data received
    InvalidData,
    /// Invalid checksum
    InvalidChecksum,

    /// I/O error
    IoError(io::Error),

    /// Checksum error
    ChecksumError,
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
            AutomotiveError::DoIPError(msg) => write!(f, "DoIP error: {}", msg),
            AutomotiveError::ConnectionFailed => write!(f, "DoIP connection failed"),
            AutomotiveError::SendFailed => write!(f, "DoIP send failed"),
            AutomotiveError::ReceiveFailed => write!(f, "DoIP receive failed"),
            AutomotiveError::Timeout => write!(f, "Operation timed out"),
            AutomotiveError::BufferOverflow => write!(f, "Buffer overflow"),
            AutomotiveError::InvalidParameter => write!(f, "Invalid parameter"),
            AutomotiveError::NotInitialized => write!(f, "Component not initialized"),
            AutomotiveError::PortError(msg) => write!(f, "Port error: {}", msg),
            AutomotiveError::InvalidData => write!(f, "Invalid data received"),
            AutomotiveError::InvalidChecksum => write!(f, "Invalid checksum"),
            AutomotiveError::IoError(err) => write!(f, "I/O error: {}", err),
            AutomotiveError::ChecksumError => write!(f, "Checksum error"),
        }
    }
}

impl Error for AutomotiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AutomotiveError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for AutomotiveError {
    fn from(err: io::Error) -> Self {
        AutomotiveError::IoError(err)
    }
}

/// Result type alias for automotive operations
pub type Result<T> = std::result::Result<T, AutomotiveError>;
