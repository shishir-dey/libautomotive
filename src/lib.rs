//! # libautomotive
//!
//! `libautomotive` is a comprehensive Rust library for automotive protocol implementations,
//! following the OSI layer model for clear separation of concerns. It provides support for
//! various automotive protocols including CAN, CAN-FD, ISO-TP, J1939, UDS, and OBD-II.
//!
//! ## Architecture
//!
//! The library is organized according to the OSI layer model:
//!
//! - Physical Layer: CAN and CAN-FD implementations
//! - Data Link Layer: Raw CAN frame handling
//! - Network Layer: J1939 protocol implementation
//! - Transport Layer: ISO-TP (ISO 15765-2) implementation
//! - Application Layer: UDS (ISO 14229) and OBD-II implementations
//!
//! ## Features
//!
//! - Complete automotive protocol stack
//! - Modular and extensible design
//! - High-performance implementations
//! - Strong type safety and error handling
//! - Easy-to-use abstractions
//!
//! ## Example
//!
//! ```text
//! # Complete usage example (conceptual, not actual code)
//! use libautomotive::physical::can;
//! use libautomotive::transport::isotp;
//! use libautomotive::application::uds;
//!
//! # 1. Set up physical layer (CAN)
//! let can_config = can::CustomConfig {
//!     bitrate: 500_000,
//!     sample_point: 0.75
//! };
//! let mut can = can::CustomInterface::new(can_config);
//! can.open();
//!
//! # 2. Set up transport layer (ISO-TP)
//! let isotp_config = isotp::CustomConfig {
//!     tx_id: 0x7E0,
//!     rx_id: 0x7E8,
//!     block_size: 8,
//!     st_min: 10
//! };
//! let mut isotp = isotp::CustomInterface::new_with_can(isotp_config, can);
//! isotp.open();
//!
//! # 3. Set up application layer (UDS)
//! let uds_config = uds::CustomConfig {
//!     timeout_ms: 1000,
//!     p2_timeout_ms: 5000
//! };
//! let mut uds = uds::CustomInterface::new_with_isotp(uds_config, isotp);
//! uds.open();
//!
//! # 4. Use UDS services
//! uds.change_session(uds::SESSION_EXTENDED);
//! let vin = uds.read_data_by_id(0xF190);  // Read Vehicle Identification Number
//! ```
//!
//! ## Credits and Acknowledgments
//!
//! This library draws inspiration from and acknowledges the following open-source projects:
//!
//! - [esp32-isotp-ble-bridge](https://github.com/bri3d/esp32-isotp-ble-bridge) - ESP32-IDF based BLE<->ISO-TP bridge
//! - [Open-SAE-J1939](https://github.com/DanielMartensson/Open-SAE-J1939) - Open source SAE J1939 implementation
//! - [uds-c](https://github.com/openxc/uds-c) - Unified Diagnostic Services (UDS) C library
//! - [obdii](https://github.com/ejvaughan/obdii) - OBD-II diagnostic protocol implementation
//! - [canis-can-sdk](https://github.com/kentindell/canis-can-sdk) - CAN protocol stack implementation
//! - [AgIsoStack++](https://github.com/Open-Agriculture/AgIsoStack-plus-plus) - Open-source C++ ISOBUS library
//! - [open-LIN-c](https://github.com/open-LIN/open-LIN-c) - Implementation of Local Interconnect Network in C
//! - [doip-library](https://github.com/doip/doip-library) - Diagnostic over IP (DoIP) protocol implementation
//!
//! These projects have provided valuable insights and reference implementations for various
//! automotive protocols. We are grateful to their authors and contributors for making their
//! work available to the community.

// OSI Layer modules
/// Application layer protocols including UDS and OBD-II
pub mod application;
/// Data link layer handling raw CAN frames
pub mod data_link; // Raw CAN frame handling
/// Network layer implementing J1939 protocol
pub mod network; // J1939 implementation
/// Physical layer implementations for CAN and CAN-FD
pub mod physical; // CAN, CANFD implementations
/// Transport layer implementing ISO-TP (ISO 15765-2)
pub mod transport; // ISO-TP implementation // UDS and OBD-II implementations

// Re-exports for convenience
pub use application::{obdii, uds};
pub use network::j1939;
pub use physical::{can, canfd};
pub use transport::isotp;

// Common types and traits
/// Common error types and error handling functionality
pub mod error;
/// Common types used across the library
pub mod types;

// Version information
/// Current version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_valid() {
        assert!(!VERSION.is_empty());
    }
}
