//! Physical layer implementations for automotive protocols.
//!
//! This module provides implementations for the physical layer protocols:
//! - Classic CAN (Controller Area Network)
//! - CAN-FD (CAN with Flexible Data-Rate)
//!
//! The physical layer is responsible for the actual transmission and reception
//! of bits on the physical medium. It handles:
//! - Bit timing and synchronization
//! - Signal levels and electrical characteristics
//! - Frame formatting and bit stuffing
//! - Error detection at the physical level
//!
//! # Examples
//!
//! ```text
//! # Example usage of the physical layer (conceptual, not actual code)
//! # CAN interface example:
//! let can_config = can::CustomConfig {
//!     bitrate: 500_000,  // 500 kbps
//!     sample_point: 0.75  // 75% sample point
//! };
//! let mut can = can::CustomInterface::new(can_config);
//! can.open();
//!
//! # CAN-FD interface example:
//! let canfd_config = canfd::CustomConfig {
//!     data_bitrate: 2_000_000,  // 2 Mbps
//!     nominal_bitrate: 500_000  // 500 kbps
//! };
//! let mut canfd = canfd::CustomInterface::new(canfd_config);
//! canfd.open();
//! ```

pub mod can;
pub mod canfd;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

use crate::error::{AutomotiveError, Result};
use crate::types::{Config, Frame};

/// Physical layer trait that must be implemented by hardware interfaces
pub trait PhysicalLayer: Send + Sync {
    type Config: Config;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn send_frame(&mut self, frame: &Frame) -> Result<()>;
    fn receive_frame(&mut self) -> Result<Frame>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
}
