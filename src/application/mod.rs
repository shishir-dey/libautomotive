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
//! ```rust,no_run
//! use libautomotive::application::{uds, obdii};
//!
//! // UDS example
//! let uds_config = uds::Config::default();
//! let uds_interface = uds::Interface::new(uds_config);
//!
//! // OBD-II example
//! let obd_config = obdii::Config::default();
//! let obd_interface = obdii::Interface::new(obd_config);
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
