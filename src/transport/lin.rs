use crate::error::{AutomotiveError, Result};
use crate::physical::PhysicalLayer;
use crate::transport::TransportLayer;
use crate::types::{Config, Frame};

// LIN constants
pub const LIN_SYNC_BYTE: u8 = 0x55;
pub const LIN_ID_MASK: u8 = 0x3F;
pub const LIN_P0_FLAG: u8 = 6;
pub const LIN_P1_FLAG: u8 = 7;
pub const LIN_BREAK_BYTE: u8 = 0x00;

// LIN frame types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinFrameType {
    Classic,
    Enhanced,
}

// LIN states
#[derive(Debug, Clone, Copy, PartialEq)]
enum LinState {
    Idle,
    Break,
    Sync,
    Id,
    Data,
    Checksum,
}

// LIN frame slot
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinFrameSlot {
    Unconditional,
    Event,
    Sporadic,
    Diagnostic,
}

#[derive(Debug, Clone)]
pub struct LinConfig {
    pub timeout_ms: u32,
    pub frame_type: LinFrameType,
}

impl Config for LinConfig {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

impl Default for LinConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 1000,
            frame_type: LinFrameType::Classic,
        }
    }
}

pub struct Lin<P: PhysicalLayer> {
    config: LinConfig,
    physical: P,
    is_open: bool,
    state: LinState,
}

impl<P: PhysicalLayer> Lin<P> {
    pub fn with_physical(config: LinConfig, physical: P) -> Self {
        Self {
            config,
            physical,
            is_open: false,
            state: LinState::Idle,
        }
    }

    /// Sends a LIN header (break, sync, and ID)
    pub fn send_header(&mut self, pid: u8) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        // Send break
        self.physical.send_frame(&Frame {
            id: 0,
            data: vec![LIN_BREAK_BYTE],
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })?;

        // Send sync
        self.physical.send_frame(&Frame {
            id: 0,
            data: vec![LIN_SYNC_BYTE],
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })?;

        // Send PID
        let parity = calculate_parity(pid);
        let pid_with_parity = pid | parity;
        self.physical.send_frame(&Frame {
            id: 0,
            data: vec![pid_with_parity],
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })?;

        Ok(())
    }

    /// Sends a LIN response (data and checksum)
    pub fn send_response(&mut self, pid: u8, data: &[u8]) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        if data.len() > 8 {
            return Err(AutomotiveError::InvalidParameter);
        }

        // Send data
        self.physical.send_frame(&Frame {
            id: 0,
            data: data.to_vec(),
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })?;

        // Calculate and send checksum
        let checksum = if self.config.frame_type == LinFrameType::Enhanced {
            calculate_enhanced_checksum(pid, data)
        } else {
            calculate_classic_checksum(data)
        };

        self.physical.send_frame(&Frame {
            id: 0,
            data: vec![checksum],
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })?;

        Ok(())
    }

    /// Reads a LIN response
    pub fn read_response(&mut self, timeout_ms: u32) -> Result<Vec<u8>> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        let mut response = Vec::new();
        let mut checksum = None;

        // Read data bytes
        for _ in 0..8 {
            match self.physical.receive_frame() {
                Ok(frame) => {
                    if frame.data.is_empty() {
                        break;
                    }
                    response.extend_from_slice(&frame.data);
                }
                Err(AutomotiveError::Timeout) => break,
                Err(e) => return Err(e),
            }
        }

        // Read checksum
        match self.physical.receive_frame() {
            Ok(frame) => {
                if !frame.data.is_empty() {
                    checksum = Some(frame.data[0]);
                }
            }
            Err(AutomotiveError::Timeout) => {}
            Err(e) => return Err(e),
        }

        // Verify checksum if received
        if let Some(received_checksum) = checksum {
            let expected_checksum = if self.config.frame_type == LinFrameType::Enhanced {
                calculate_enhanced_checksum(0, &response) // PID not available here
            } else {
                calculate_classic_checksum(&response)
            };

            if received_checksum != expected_checksum {
                return Err(AutomotiveError::ChecksumError);
            }
        }

        Ok(response)
    }
}

impl<P: PhysicalLayer> TransportLayer for Lin<P> {
    type Config = LinConfig;

    fn new(_config: Self::Config) -> Result<Self> {
        Err(AutomotiveError::NotInitialized) // Requires physical layer
    }

    fn open(&mut self) -> Result<()> {
        if self.is_open {
            return Ok(());
        }
        self.physical.set_timeout(self.config.timeout_ms)?;
        self.is_open = true;
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        self.is_open = false;
        Ok(())
    }

    fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }
        self.physical.send_frame(frame)
    }

    fn read_frame(&mut self) -> Result<Frame> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }
        self.physical.receive_frame()
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }
        self.physical.set_timeout(timeout_ms)
    }
}

// Helper functions for LIN protocol

fn calculate_parity(pid: u8) -> u8 {
    let p0 = (pid ^ (pid >> 1) ^ (pid >> 2) ^ (pid >> 4)) & 1;
    let p1 = !((pid >> 1) ^ (pid >> 3) ^ (pid >> 4) ^ (pid >> 5)) & 1;
    (p0 << 6) | (p1 << 7)
}

fn calculate_classic_checksum(data: &[u8]) -> u8 {
    let mut sum: u16 = 0;
    for &byte in data {
        sum = sum.wrapping_add(byte as u16);
        if sum > 0xFF {
            sum = (sum & 0xFF) + 1;
        }
    }
    (!sum as u8)
}

fn calculate_enhanced_checksum(pid: u8, data: &[u8]) -> u8 {
    let mut sum: u16 = pid as u16;
    for &byte in data {
        sum = sum.wrapping_add(byte as u16);
        if sum > 0xFF {
            sum = (sum & 0xFF) + 1;
        }
    }
    (!sum as u8)
}
