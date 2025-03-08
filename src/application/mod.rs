//! Application layer implementations for automotive protocols.
//!
//! This module provides implementations for application layer protocols:
//! - UDS (ISO 14229-1) - Unified Diagnostic Services
//! - OBD-II (ISO 15031) - On-Board Diagnostics
//!
//! The application layer provides high-level diagnostic and monitoring services:
//!
//! ## UDS (Unified Diagnostic Services)
//! - ECU programming and configuration
//! - Diagnostic trouble code (DTC) management
//! - Data reading and writing
//! - Routine control
//! - Security access
//!
//! ## OBD-II (On-Board Diagnostics)
//! - Emissions-related diagnostics
//! - Real-time parameter monitoring
//! - Freeze frame data access
//! - Standard diagnostic trouble codes
//! - Mode-based service requests
//!
//! # Examples
//!
//! ```text
//! # Example usage of the application layer (conceptual, not actual code)
//! # UDS configuration:
//! let uds_config = uds::CustomConfig {
//!     timeout_ms: 1000,             // 1 second timeout
//!     p2_timeout_ms: 5000,          // 5 second P2 timeout
//!     tester_present_interval_ms: 2000  // Send tester present every 2 seconds
//! };
//!
//! # Create a UDS instance
//! let mut uds = uds::CustomInterface::new(uds_config);
//! uds.open();
//!
//! # Change diagnostic session
//! uds.change_session(uds::SESSION_PROGRAMMING);
//!
//! # Read ECU data
//! let vin_data = uds.read_data_by_id(0xF190); // Vehicle Identification Number
//!
//! # Write ECU data
//! uds.write_data_by_id(0xF198, &[0x01, 0x02, 0x03]);
//!
//! # OBD-II configuration:
//! let obd_config = obdii::CustomConfig {
//!     timeout_ms: 1000,    // 1 second timeout
//!     auto_format: true    // Auto-format responses
//! };
//!
//! # Create an OBD-II instance
//! let mut obd = obdii::CustomInterface::new(obd_config);
//! obd.open();
//!
//! # Read engine RPM
//! let engine_rpm = obd.read_sensor(0x0C); // PID for engine RPM
//!
//! # Read diagnostic trouble codes
//! let dtcs = obd.read_dtc();
//! ```

pub mod obdii;
pub mod uds;

use crate::error::Result;
use crate::types::Config;

pub use obdii::Obd;
pub use uds::Uds;

/// Application layer trait that must be implemented by UDS and OBD-II
pub trait ApplicationLayer {
    type Config: Config;
    type Request;
    type Response;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn send_request(&mut self, request: &Self::Request) -> Result<Self::Response>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
}

#[cfg(test)]
mod tests;
