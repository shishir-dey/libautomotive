use super::TransportLayer;
use crate::error::{AutomotiveError, Result};
use crate::physical::PhysicalLayer;
use crate::transport::IsoTpTransport;
use crate::types::{Config, Frame};

const SF_PCI: u8 = 0x00; // Single Frame
const FF_PCI: u8 = 0x10; // First Frame
const CF_PCI: u8 = 0x20; // Consecutive Frame
const FC_PCI: u8 = 0x30; // Flow Control

/// ISO-TP Address Modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AddressMode {
    Normal,
    Extended,
    Mixed,
}

/// ISO-TP Timing Parameters (in milliseconds)
#[derive(Debug, Clone)]
pub struct IsoTpTiming {
    pub n_as: u32, // Sender N_As timeout
    pub n_ar: u32, // Receiver N_Ar timeout
    pub n_bs: u32, // Sender N_Bs timeout
    pub n_cr: u32, // Receiver N_Cr timeout
}

impl Default for IsoTpTiming {
    fn default() -> Self {
        Self {
            n_as: 1000, // Default 1 second
            n_ar: 1000,
            n_bs: 1000,
            n_cr: 1000,
        }
    }
}

/// ISO-TP configuration
#[derive(Debug, Clone)]
pub struct IsoTpConfig {
    pub tx_id: u32,
    pub rx_id: u32,
    pub block_size: u8,
    pub st_min: u8,
    pub address_mode: AddressMode,
    pub address_extension: u8,
    pub use_padding: bool,
    pub padding_value: u8,
    pub timing: IsoTpTiming,
    pub timeout_ms: u32,
}

impl Config for IsoTpConfig {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

impl Default for IsoTpConfig {
    fn default() -> Self {
        Self {
            tx_id: 0,
            rx_id: 0,
            block_size: 0,
            st_min: 0,
            address_mode: AddressMode::Normal,
            address_extension: 0,
            use_padding: false,
            padding_value: 0x00,
            timing: IsoTpTiming::default(),
            timeout_ms: 1000,
        }
    }
}

/// ISO-TP implementation
pub struct IsoTp<P: PhysicalLayer> {
    config: IsoTpConfig,
    physical: P,
    is_open: bool,
}

impl<P: PhysicalLayer> IsoTp<P> {
    /// Creates a new ISO-TP instance with the given physical layer
    pub fn with_physical(config: IsoTpConfig, physical: P) -> Self {
        Self {
            config,
            physical,
            is_open: false,
        }
    }

    fn send_single_frame(&mut self, data: &[u8]) -> Result<()> {
        let mut frame_data = vec![];

        // Add address extension if needed
        if self.config.address_mode == AddressMode::Extended {
            frame_data.push(self.config.address_extension);
        }

        // Add PCI and data
        frame_data.push(data.len() as u8);
        frame_data.extend_from_slice(data);

        // Add padding if configured
        if self.config.use_padding {
            while frame_data.len() < 8 {
                frame_data.push(self.config.padding_value);
            }
        }

        self.write_frame(&Frame {
            id: if self.config.address_mode == AddressMode::Mixed {
                self.config.tx_id | (self.config.address_extension as u32)
            } else {
                self.config.tx_id
            },
            data: frame_data,
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    }

    fn send_multi_frame(&mut self, data: &[u8]) -> Result<()> {
        // First frame
        let mut frame_data = vec![];

        // Add address extension if needed
        if self.config.address_mode == AddressMode::Extended {
            frame_data.push(self.config.address_extension);
        }

        // Add PCI and data
        frame_data.push(0x10 | ((data.len() >> 8) as u8 & 0x0F));
        frame_data.push(data.len() as u8);
        let first_data_size = if self.config.address_mode == AddressMode::Extended {
            5
        } else {
            6
        };
        frame_data.extend_from_slice(&data[0..first_data_size]);

        // Add padding if configured
        if self.config.use_padding {
            while frame_data.len() < 8 {
                frame_data.push(self.config.padding_value);
            }
        }

        self.write_frame(&Frame {
            id: if self.config.address_mode == AddressMode::Mixed {
                self.config.tx_id | (self.config.address_extension as u32)
            } else {
                self.config.tx_id
            },
            data: frame_data,
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })?;

        // Wait for flow control
        let start_time = std::time::SystemTime::now();
        loop {
            let frame = self.read_frame()?;
            if frame.data[0] == 0x30 {
                break;
            }
            if start_time.elapsed().unwrap().as_millis() as u32 > self.config.timing.n_bs {
                return Err(AutomotiveError::Timeout);
            }
        }

        // Consecutive frames
        let mut index = first_data_size;
        let mut sequence = 1;
        while index < data.len() {
            let remaining = data.len() - index;
            let chunk_size = if self.config.address_mode == AddressMode::Extended {
                remaining.min(6)
            } else {
                remaining.min(7)
            };

            let mut frame_data = vec![];

            // Add address extension if needed
            if self.config.address_mode == AddressMode::Extended {
                frame_data.push(self.config.address_extension);
            }

            // Add PCI and data
            frame_data.push(0x20 | (sequence & 0x0F));
            frame_data.extend_from_slice(&data[index..index + chunk_size]);

            // Add padding if configured
            if self.config.use_padding {
                while frame_data.len() < 8 {
                    frame_data.push(self.config.padding_value);
                }
            }

            self.write_frame(&Frame {
                id: if self.config.address_mode == AddressMode::Mixed {
                    self.config.tx_id | (self.config.address_extension as u32)
                } else {
                    self.config.tx_id
                },
                data: frame_data,
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })?;

            // Apply separation time if configured
            if self.config.st_min > 0 {
                std::thread::sleep(std::time::Duration::from_millis(self.config.st_min as u64));
            }

            index += chunk_size;
            sequence = (sequence + 1) & 0x0F;
        }
        Ok(())
    }

    fn receive_single_frame(&mut self, frame: &Frame) -> Result<Vec<u8>> {
        let data_start = if self.config.address_mode == AddressMode::Extended {
            1
        } else {
            0
        };
        let length = frame.data[data_start] & 0x0F;
        if length as usize > frame.data.len() - data_start - 1 {
            return Err(AutomotiveError::InvalidParameter);
        }
        Ok(frame.data[data_start + 1..=data_start + length as usize].to_vec())
    }

    fn receive_multi_frame(&mut self, frame: &Frame) -> Result<Vec<u8>> {
        let data_start = if self.config.address_mode == AddressMode::Extended {
            1
        } else {
            0
        };
        let length =
            ((frame.data[data_start] as usize & 0x0F) << 8) | frame.data[data_start + 1] as usize;
        let mut data = Vec::with_capacity(length);
        data.extend_from_slice(&frame.data[data_start + 2..]);

        // Send flow control
        let mut fc_data = vec![];
        if self.config.address_mode == AddressMode::Extended {
            fc_data.push(self.config.address_extension);
        }
        fc_data.extend_from_slice(&[0x30, self.config.block_size, self.config.st_min]);

        self.write_frame(&Frame {
            id: if self.config.address_mode == AddressMode::Mixed {
                self.config.tx_id | (self.config.address_extension as u32)
            } else {
                self.config.tx_id
            },
            data: fc_data,
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })?;

        let mut sequence = 1;
        while data.len() < length {
            let frame = self.read_frame()?;
            if frame.data.is_empty() {
                return Err(AutomotiveError::InvalidParameter);
            }

            let data_start = if self.config.address_mode == AddressMode::Extended {
                1
            } else {
                0
            };
            if frame.data[data_start] & 0xF0 != 0x20 {
                return Err(AutomotiveError::InvalidParameter);
            }
            if frame.data[data_start] & 0x0F != sequence {
                return Err(AutomotiveError::InvalidParameter);
            }
            data.extend_from_slice(&frame.data[data_start + 1..]);
            sequence = (sequence + 1) & 0x0F;
        }
        data.truncate(length);
        Ok(data)
    }
}

impl<P: PhysicalLayer> TransportLayer for IsoTp<P> {
    type Config = IsoTpConfig;

    fn new(_config: Self::Config) -> Result<Self> {
        Err(AutomotiveError::NotInitialized) // Requires physical layer
    }

    fn open(&mut self) -> Result<()> {
        if self.is_open {
            return Ok(());
        }
        self.physical.set_timeout(self.config.timing.n_as)?;
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

impl<P: PhysicalLayer> IsoTpTransport for IsoTp<P> {
    fn send(&mut self, data: &[u8]) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }
        if data.is_empty() {
            return Err(AutomotiveError::InvalidParameter);
        }
        if data.len() <= 7 {
            self.send_single_frame(data)
        } else {
            self.send_multi_frame(data)
        }
    }

    fn receive(&mut self) -> Result<Vec<u8>> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }
        let frame = self.read_frame()?;
        if frame.data.is_empty() {
            return Err(AutomotiveError::InvalidParameter);
        }
        let data_start = if self.config.address_mode == AddressMode::Extended {
            1
        } else {
            0
        };
        match frame.data[data_start] & 0xF0 {
            0x00 => self.receive_single_frame(&frame),
            0x10 => self.receive_multi_frame(&frame),
            _ => Err(AutomotiveError::InvalidParameter),
        }
    }
}
