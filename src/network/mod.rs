//! Network layer implementations for automotive protocols.
//!
//! This module provides implementations for network layer protocols, primarily:
//! - J1939 (SAE J1939) - A higher-layer protocol for commercial vehicles
//!
//! The network layer is responsible for:
//! - Message routing and addressing
//! - Network management
//! - Message prioritization
//! - Protocol-specific addressing schemes
//!
//! The J1939 protocol is widely used in commercial vehicles and provides:
//! - Parameter Group Numbers (PGN) based addressing
//! - Multi-packet message transport
//! - Network management functions
//! - Standardized diagnostic messages
//!
//! # Examples
//!
//! ```rust,no_run
//! use libautomotive::network::j1939;
//!
//! let config = j1939::Config::default();
//! let interface = j1939::Interface::new(config);
//!
//! // Send a J1939 message
//! let msg = j1939::Message::new(/* ... */);
//! interface.send(&msg);
//! ```

pub mod j1939;

use crate::error::Result;
use crate::types::{Address, Config};

/// Network layer trait that must be implemented by J1939
pub trait NetworkLayer: Send + Sync {
    type Config: Config;
    type Message;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn send(&mut self, address: &Address, data: &[u8]) -> Result<()>;
    fn receive(&mut self) -> Result<Self::Message>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
    fn claim_address(&mut self, address: u8) -> Result<()>;
    fn get_address(&self) -> Result<u8>;
}
