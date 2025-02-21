//! Common types used throughout the automotive protocol stack.
//!
//! This module provides fundamental types and traits that are used across different
//! layers of the protocol stack. It includes basic types for CAN communication,
//! addressing, and frame structures, as well as traits for configuration and
//! hardware port interactions.

/// CAN identifier type, supporting both standard (11-bit) and extended (29-bit) identifiers.
pub type CanId = u32;

/// Frame data type representing the payload of a CAN frame.
///
/// The maximum length depends on the protocol:
/// - Classic CAN: 8 bytes
/// - CAN-FD: up to 64 bytes
pub type FrameData = Vec<u8>;

/// Timestamp type representing milliseconds since an arbitrary epoch.
///
/// Used for timing and synchronization purposes across the protocol stack.
pub type Timestamp = u64;

/// Protocol-specific addressing information, primarily used in higher layer protocols
/// like J1939.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address {
    /// Message priority (0-7, with 0 being highest priority)
    pub priority: u8,
    /// Parameter Group Number (PGN) identifying the message type
    pub pgn: u32,
    /// Source address of the sending node
    pub source: u8,
    /// Destination address of the target node
    pub destination: u8,
}

/// Generic frame structure used across different protocol layers.
///
/// This structure provides a unified representation of CAN frames,
/// supporting both classic CAN and CAN-FD formats.
#[derive(Debug, Clone)]
pub struct Frame {
    /// CAN identifier (11-bit or 29-bit)
    pub id: CanId,
    /// Frame payload data
    pub data: FrameData,
    /// Timestamp of frame reception/transmission
    pub timestamp: Timestamp,
    /// Whether the frame uses extended (29-bit) identifier
    pub is_extended: bool,
    /// Whether the frame is a CAN-FD frame
    pub is_fd: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            id: 0,
            data: Vec::new(),
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        }
    }
}

/// Configuration trait that must be implemented by all protocol configurations.
///
/// This trait ensures that protocol configurations can be validated before use
/// and can be safely shared between threads.
pub trait Config: Send + Sync {
    /// Validates the configuration parameters.
    ///
    /// Returns `Ok(())` if the configuration is valid, or an appropriate error
    /// if validation fails.
    fn validate(&self) -> crate::error::Result<()>;
}

/// Hardware abstraction trait for CAN interfaces.
///
/// This trait must be implemented by platform-specific code to provide
/// the actual hardware communication capabilities.
pub trait Port: Send + Sync {
    /// Sends a frame through the CAN interface.
    fn send(&mut self, frame: &Frame) -> crate::error::Result<()>;
    
    /// Receives a frame from the CAN interface.
    ///
    /// This method will block until a frame is received or a timeout occurs.
    fn receive(&mut self) -> crate::error::Result<Frame>;
    
    /// Sets the timeout for receive operations.
    ///
    /// # Parameters
    /// * `timeout_ms` - Timeout in milliseconds. A value of 0 means no timeout.
    fn set_timeout(&mut self, timeout_ms: u32) -> crate::error::Result<()>;
}
