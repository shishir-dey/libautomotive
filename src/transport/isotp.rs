use super::TransportLayer;
use crate::error::{AutomotiveError, Result};
use crate::physical::PhysicalLayer;
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
    pub max_frame_size: usize,
    pub address_mode: AddressMode,
    pub address_extension: u8,
    pub padding_value: u8,
    pub use_padding: bool,
    pub timing: IsoTpTiming,
}

impl Default for IsoTpConfig {
    fn default() -> Self {
        Self {
            tx_id: 0,
            rx_id: 0,
            block_size: 0,
            st_min: 0,
            max_frame_size: 4095, // Default max ISO-TP message size
            address_mode: AddressMode::Normal,
            address_extension: 0,
            padding_value: 0x55,
            use_padding: false,
            timing: IsoTpTiming::default(),
        }
    }
}

impl Config for IsoTpConfig {
    fn validate(&self) -> Result<()> {
        if self.tx_id == self.rx_id {
            return Err(AutomotiveError::InvalidParameter);
        }
        if self.max_frame_size < 8 {
            return Err(AutomotiveError::InvalidParameter);
        }
        Ok(())
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
        let mut frame = Frame {
            id: self.config.tx_id,
            data: vec![SF_PCI | (data.len() as u8)],
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        };
        frame.data.extend_from_slice(data);
        self.apply_padding(&mut frame.data);
        self.send_frame_with_addressing(frame)
    }

    fn send_multi_frame(&mut self, data: &[u8]) -> Result<()> {
        // Send First Frame
        let len = data.len();
        let mut frame = Frame {
            id: self.config.tx_id,
            data: vec![FF_PCI | ((len >> 8) as u8), len as u8],
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        };
        frame.data.extend_from_slice(&data[0..6]);
        self.apply_padding(&mut frame.data);
        self.send_frame_with_addressing(frame)?;

        // Wait for Flow Control with timeout
        let start_time = std::time::Instant::now();
        let mut fc = None;
        while start_time.elapsed().as_millis() < self.config.timing.n_bs as u128 {
            match self.receive_frame_with_addressing() {
                Ok(frame) => {
                    if !frame.data.is_empty() && frame.data[0] & 0xF0 == FC_PCI {
                        fc = Some(frame);
                        break;
                    }
                }
                Err(_) => continue,
            }
        }

        let fc = fc.ok_or_else(|| AutomotiveError::IsoTpError("Flow control timeout".into()))?;
        let block_size = fc.data[1];
        let st_min = fc.data[2];

        // Send Consecutive Frames
        let mut index = 6;
        let mut sequence = 1u8;
        let mut frames_sent = 0;

        while index < len {
            let remaining = len - index;
            let chunk_size = remaining.min(7);

            let mut frame = Frame {
                id: self.config.tx_id,
                data: vec![CF_PCI | sequence],
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            };
            frame
                .data
                .extend_from_slice(&data[index..index + chunk_size]);
            self.apply_padding(&mut frame.data);
            self.send_frame_with_addressing(frame)?;

            frames_sent += 1;

            // Check if we need to wait for next flow control
            if block_size > 0 && frames_sent == block_size as usize {
                frames_sent = 0;

                // Wait for next Flow Control
                let start_time = std::time::Instant::now();
                let mut next_fc = None;
                while start_time.elapsed().as_millis() < self.config.timing.n_bs as u128 {
                    match self.receive_frame_with_addressing() {
                        Ok(frame) => {
                            if !frame.data.is_empty() && frame.data[0] & 0xF0 == FC_PCI {
                                next_fc = Some(frame);
                                break;
                            }
                        }
                        Err(_) => continue,
                    }
                }

                next_fc
                    .ok_or_else(|| AutomotiveError::IsoTpError("Flow control timeout".into()))?;
            }

            // Respect separation time
            if st_min <= 0x7F {
                std::thread::sleep(std::time::Duration::from_millis(st_min as u64));
            } else if st_min >= 0xF1 && st_min <= 0xF9 {
                std::thread::sleep(std::time::Duration::from_micros(
                    ((st_min - 0xF0) * 100) as u64,
                ));
            }

            index += chunk_size;
            sequence = (sequence + 1) & 0x0F;
        }

        Ok(())
    }

    fn send_frame_with_addressing(&mut self, mut frame: Frame) -> Result<()> {
        match self.config.address_mode {
            AddressMode::Normal => self.physical.send_frame(&frame),
            AddressMode::Extended => {
                let mut data = vec![self.config.address_extension];
                data.extend(frame.data);
                frame.data = data;
                self.physical.send_frame(&frame)
            }
            AddressMode::Mixed => {
                frame.id = (frame.id & 0xFFFFFF00) | (self.config.address_extension as u32);
                self.physical.send_frame(&frame)
            }
        }
    }

    fn receive_frame_with_addressing(&mut self) -> Result<Frame> {
        let frame = self.physical.receive_frame()?;

        match self.config.address_mode {
            AddressMode::Normal => Ok(frame),
            AddressMode::Extended => {
                if frame.data.is_empty() {
                    return Err(AutomotiveError::IsoTpError(
                        "Empty frame in extended addressing".into(),
                    ));
                }
                if frame.data[0] != self.config.address_extension {
                    return Err(AutomotiveError::IsoTpError(
                        "Invalid address extension".into(),
                    ));
                }
                Ok(Frame {
                    data: frame.data[1..].to_vec(),
                    ..frame
                })
            }
            AddressMode::Mixed => {
                let received_ae = (frame.id & 0xFF) as u8;
                if received_ae != self.config.address_extension {
                    return Err(AutomotiveError::IsoTpError(
                        "Invalid address extension".into(),
                    ));
                }
                Ok(frame)
            }
        }
    }

    fn apply_padding(&self, data: &mut Vec<u8>) {
        if self.config.use_padding {
            while data.len() < 8 {
                data.push(self.config.padding_value);
            }
        }
    }

    fn receive_single_frame(&mut self, frame: &Frame) -> Result<Vec<u8>> {
        let len = (frame.data[0] & 0x0F) as usize;
        if len == 0 || len > frame.data.len() - 1 {
            return Err(AutomotiveError::IsoTpError("Invalid SF length".into()));
        }
        Ok(frame.data[1..len + 1].to_vec())
    }

    fn receive_multi_frame(&mut self, first_frame: &Frame) -> Result<Vec<u8>> {
        let mut len = ((first_frame.data[0] & 0x0F) as usize) << 8;
        len |= first_frame.data[1] as usize;

        if len < 8 {
            return Err(AutomotiveError::IsoTpError("Invalid FF length".into()));
        }

        let mut data = Vec::with_capacity(len);
        data.extend_from_slice(&first_frame.data[2..8]);

        // Send Flow Control
        let mut fc_frame = Frame {
            id: self.config.tx_id,
            data: vec![FC_PCI, self.config.block_size, self.config.st_min],
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        };
        self.apply_padding(&mut fc_frame.data);
        self.send_frame_with_addressing(fc_frame)?;

        // Receive Consecutive Frames
        let mut expected_sequence = 1u8;
        let mut frames_received = 0;

        while data.len() < len {
            // Wait for next frame with timeout
            let start_time = std::time::Instant::now();
            let mut cf = None;
            while start_time.elapsed().as_millis() < self.config.timing.n_cr as u128 {
                match self.receive_frame_with_addressing() {
                    Ok(frame) => {
                        if !frame.data.is_empty() && frame.data[0] & 0xF0 == CF_PCI {
                            cf = Some(frame);
                            break;
                        }
                    }
                    Err(_) => continue,
                }
            }

            let cf =
                cf.ok_or_else(|| AutomotiveError::IsoTpError("Consecutive frame timeout".into()))?;
            let sequence = cf.data[0] & 0x0F;

            if sequence != expected_sequence {
                return Err(AutomotiveError::IsoTpError("Wrong sequence number".into()));
            }

            let remaining = len - data.len();
            let chunk_size = remaining.min(7);
            data.extend_from_slice(&cf.data[1..chunk_size + 1]);

            frames_received += 1;

            // Send flow control if needed
            if self.config.block_size > 0 && frames_received == self.config.block_size as usize {
                frames_received = 0;
                let mut fc_frame = Frame {
                    id: self.config.tx_id,
                    data: vec![FC_PCI, self.config.block_size, self.config.st_min],
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                };
                self.apply_padding(&mut fc_frame.data);
                self.send_frame_with_addressing(fc_frame)?;
            }

            expected_sequence = (expected_sequence + 1) & 0x0F;
        }

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

        self.config.validate()?;
        self.physical.open()?;
        self.is_open = true;
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        if !self.is_open {
            return Ok(());
        }

        self.physical.close()?;
        self.is_open = false;
        Ok(())
    }

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

        let start_time = std::time::Instant::now();
        let mut frame = None;
        while start_time.elapsed().as_millis() < self.config.timing.n_ar as u128 {
            match self.receive_frame_with_addressing() {
                Ok(f) => {
                    frame = Some(f);
                    break;
                }
                Err(_) => continue,
            }
        }

        let frame = frame.ok_or_else(|| AutomotiveError::IsoTpError("Receive timeout".into()))?;

        if frame.data.is_empty() {
            return Err(AutomotiveError::IsoTpError("Empty frame".into()));
        }

        match frame.data[0] & 0xF0 {
            SF_PCI => self.receive_single_frame(&frame),
            FF_PCI => self.receive_multi_frame(&frame),
            _ => Err(AutomotiveError::IsoTpError("Invalid PCI".into())),
        }
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        self.physical.set_timeout(timeout_ms)
    }
}
