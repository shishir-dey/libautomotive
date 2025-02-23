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
//! ```rust,no_run
//! use libautomotive::{can, j1939, uds};
//!
//! // Your implementation code here
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
