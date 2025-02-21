/// CAN ID type
pub type CanId = u32;

/// Generic frame data type
pub type FrameData = Vec<u8>;

/// Timestamp in milliseconds
pub type Timestamp = u64;

/// Protocol specific address type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address {
    pub priority: u8,
    pub pgn: u32,
    pub source: u8,
    pub destination: u8,
}

/// Generic frame structure used across layers
#[derive(Debug, Clone)]
pub struct Frame {
    pub id: CanId,
    pub data: FrameData,
    pub timestamp: Timestamp,
    pub is_extended: bool,
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

/// Configuration trait that must be implemented by all protocol configurations
pub trait Config: Send + Sync {
    fn validate(&self) -> crate::error::Result<()>;
}

/// Port trait that must be implemented by platform-specific code
pub trait Port: Send + Sync {
    fn send(&mut self, frame: &Frame) -> crate::error::Result<()>;
    fn receive(&mut self) -> crate::error::Result<Frame>;
    fn set_timeout(&mut self, timeout_ms: u32) -> crate::error::Result<()>;
}
