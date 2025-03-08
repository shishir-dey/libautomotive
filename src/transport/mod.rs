//! Transport layer implementations for automotive protocols.
//!
//! This module provides implementations for transport layer protocols, primarily:
//! - ISO-TP (ISO 15765-2) - Transport Protocol for diagnostic communication
//!
//! The transport layer is responsible for:
//! - Segmentation and reassembly of large messages
//! - Flow control
//! - Error recovery
//! - End-to-end message delivery
//!
//! ISO-TP is widely used in automotive diagnostics and provides:
//! - Single Frame (SF) for messages up to 7 bytes
//! - First Frame (FF) and Consecutive Frames (CF) for longer messages
//! - Flow Control (FC) frames for managing message transmission
//! - Support for normal and extended addressing
//!
//! # Examples
//!
//! ```text
//! # Example usage of the transport layer (conceptual, not actual code)
//! # ISO-TP configuration example:
//! let isotp_config = isotp::CustomConfig {
//!     tx_id: 0x7E0,         // Transmit CAN ID
//!     rx_id: 0x7E8,         // Receive CAN ID
//!     block_size: 8,        // Number of frames before flow control
//!     st_min: 10,           // Minimum separation time between frames (10ms)
//!     use_extended_addressing: false
//! };
//!
//! # Create an ISO-TP instance with the config
//! let mut isotp = isotp::CustomInterface::new(isotp_config);
//!
//! # Open the connection
//! isotp.open();
//!
//! # Send a diagnostic message (UDS request)
//! let request_data = [0x22, 0xF1, 0x90]; // UDS read data by ID (VIN)
//! isotp.send(&request_data);
//!
//! # Receive response
//! let response = isotp.receive();
//! ```

pub mod doip;
mod isobus;
mod isobus_diagnostic;
pub mod isotp;
pub mod lin;

use crate::error::Result;
use crate::types::{Config, Frame};

/// Base transport layer trait
pub trait TransportLayer {
    type Config: Config;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn write_frame(&mut self, frame: &Frame) -> Result<()>;
    fn read_frame(&mut self) -> Result<Frame>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
}

/// ISO-TP specific transport layer trait
pub trait IsoTpTransport: TransportLayer {
    fn send(&mut self, data: &[u8]) -> Result<()>;
    fn receive(&mut self) -> Result<Vec<u8>>;
}

pub use doip::{DoIP, DoIPConfig};
pub use isobus::{ISOBUSConfig, ISOBUS};
pub use isobus_diagnostic::{DiagnosticTroubleCode, ISOBUSDiagnosticProtocol, LampStatus};
pub use isotp::{IsoTp, IsoTpConfig};
pub use lin::{Lin, LinConfig, LinFrameSlot, LinFrameType};

#[cfg(test)]
mod tests;
