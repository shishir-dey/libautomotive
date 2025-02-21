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
//! ```rust,no_run
//! use libautomotive::transport::isotp;
//!
//! let config = isotp::Config::default();
//! let interface = isotp::Interface::new(config);
//!
//! // Send a multi-frame message
//! let data = vec![0u8; 100];
//! interface.send(&data);
//! ```

pub mod isotp;

use crate::error::Result;
use crate::types::Config;

/// Transport layer trait that must be implemented by ISO-TP
pub trait TransportLayer: Send + Sync {
    type Config: Config;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn send(&mut self, data: &[u8]) -> Result<()>;
    fn receive(&mut self) -> Result<Vec<u8>>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
}

#[cfg(test)]
mod tests;
