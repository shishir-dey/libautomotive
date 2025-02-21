// OSI Layer modules
pub mod application;
pub mod data_link; // Raw CAN frame handling
pub mod network; // J1939 implementation
pub mod physical; // CAN, CANFD implementations
pub mod transport; // ISO-TP implementation // UDS and OBD-II implementations

// Re-exports for convenience
pub use application::{obdii, uds};
pub use network::j1939;
pub use physical::{can, canfd};
pub use transport::isotp;

// Common types and traits
pub mod error;
pub mod types;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_valid() {
        assert!(!VERSION.is_empty());
    }
}
