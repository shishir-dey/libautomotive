use crate::error::Result;
use crate::types::{Config, Frame};

/// Data link layer trait for raw CAN frame handling
pub trait DataLinkLayer: Send + Sync {
    type Config: Config;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn send_frame(&mut self, frame: &Frame) -> Result<()>;
    fn receive_frame(&mut self) -> Result<Frame>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
    fn get_status(&self) -> Result<u32>;
    fn get_error_counters(&self) -> Result<(u8, u8)>; // (TEC, REC)
    fn get_bus_status(&self) -> Result<BusStatus>;
    fn request_recovery(&mut self) -> Result<()>;
}

/// CAN bus status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusStatus {
    Error,
    Warning,
    ErrorPassive,
    BusOff,
    Active,
}

/// CAN error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanErrorType {
    Bit0,      // Transmitted 0 but received 1
    Bit1,      // Transmitted 1 but received 0
    Stuff,     // Bit stuffing error
    Form,      // Form error
    Crc,       // CRC error
    Ack,       // No acknowledgment received
    Other(u8), // Other error types
}

/// CAN error frame
#[derive(Debug, Clone)]
pub struct CanError {
    pub error_type: CanErrorType,
    pub is_tx: bool,
    pub frame_type: FrameType,
    pub location: ErrorLocation,
    pub tec: u8,
    pub rec: u8,
}

/// Frame type when error occurred
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Data,
    Remote,
    Error,
    Overload,
}

/// Location in frame where error occurred
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLocation {
    Sof,
    Id,
    Rtr,
    Ide,
    R0,
    Dlc,
    Data,
    Crc,
    CrcDelimiter,
    Ack,
    AckDelimiter,
    Eof,
    InterFrame,
    Other(u8),
}
