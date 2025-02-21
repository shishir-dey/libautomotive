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
//! ```rust,no_run
//! use libautomotive::physical::{can, canfd};
//!
//! // Classic CAN example
//! let can_config = can::Config::default();
//! let can_interface = can::Interface::new(can_config);
//!
//! // CAN-FD example
//! let canfd_config = canfd::Config::default();
//! let canfd_interface = canfd::Interface::new(canfd_config);
//! ```

pub mod can;
pub mod canfd;

#[cfg(test)]
pub(crate) mod mock;

use crate::error::Result;
use crate::types::{Config, Frame};

/// Physical layer trait that must be implemented by CAN and CANFD
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
