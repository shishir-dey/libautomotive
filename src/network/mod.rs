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
//! ```text
//! # Example usage of the network layer (conceptual, not actual code)
//! # J1939 configuration:
//! let j1939_config = j1939::CustomConfig {
//!     name: 0x0000AABBCCDDEEFF, // ECU NAME (64-bit identifier)
//!     address: 0x42,            // Preferred source address
//!     priority: 6               // Default priority for messages
//! };
//!
//! # Create a J1939 instance and open the connection
//! let mut j1939 = j1939::CustomInterface::new(j1939_config);
//! j1939.open();
//!
//! # Claim a network address
//! j1939.claim_address(0x42);
//!
//! # Send a J1939 message
//! let pgn = 0x00EF00;     // Example PGN (Electronic Engine Controller)
//! let dest_addr = 0xFF;   // Broadcast address
//! let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
//!
//! j1939.send_message(pgn, dest_addr, &data);
//!
//! # Receive J1939 messages
//! let msg = j1939.receive_message();
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
