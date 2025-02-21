use super::PhysicalLayer;
use crate::error::{AutomotiveError, Result};
use crate::types::{Config, Frame, Port};
use bitflags::bitflags;

/// CAN configuration
#[derive(Debug, Clone)]
pub struct CanConfig {
    pub bitrate: u32,
    pub sample_point: f32,
    pub sjw: u8,
    pub options: CanOptions,
}

/// CAN bitrate configurations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CanBitrate {
    Rate1M,               // 1Mbit/sec, 75% sample point
    Rate500K,             // 500kbit/sec, 75% sample point
    Rate250K,             // 250kbit/sec, 75% sample point
    Rate125K,             // 125kbit/sec, 75% sample point
    Rate100K,             // 100kbit/sec, 75% sample point
    Rate50K,              // 50kbit/sec, 75% sample point
    Rate20K,              // 20kbit/sec, 75% sample point
    Rate10K,              // 10kbit/sec, 75% sample point
    Custom(u32, f32, u8), // Custom bitrate, sample point, and SJW
}

bitflags! {
    #[derive(Debug, Clone)]
    pub struct CanOptions: u32 {
        const NONE = 0;
        const LOOPBACK = 1;
        const LISTEN_ONLY = 2;
        const TRIPLE_SAMPLING = 4;
        const ONE_SHOT = 8;
        const ERR_REPORTING = 16;
    }
}

impl Config for CanConfig {
    fn validate(&self) -> Result<()> {
        if self.bitrate == 0 {
            return Err(AutomotiveError::InvalidParameter);
        }
        if self.sample_point <= 0.0 || self.sample_point >= 1.0 {
            return Err(AutomotiveError::InvalidParameter);
        }
        if self.sjw == 0 {
            return Err(AutomotiveError::InvalidParameter);
        }
        Ok(())
    }
}

/// CAN implementation
pub struct Can<P: Port> {
    config: CanConfig,
    port: P,
    is_open: bool,
    tx_queue: TxQueue,
    rx_queue: RxQueue,
    error_counters: (u8, u8), // (TEC, REC)
}

const TX_QUEUE_SIZE: usize = 32;
const RX_QUEUE_SIZE: usize = 128;

#[derive(Debug)]
struct TxQueue {
    frames: Vec<Frame>,
    head: usize,
    tail: usize,
    count: usize,
}

#[derive(Debug)]
struct RxQueue {
    frames: Vec<Frame>,
    head: usize,
    tail: usize,
    count: usize,
}

impl TxQueue {
    fn new() -> Self {
        Self {
            frames: Vec::with_capacity(TX_QUEUE_SIZE),
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push(&mut self, frame: Frame) -> Result<()> {
        if self.count >= TX_QUEUE_SIZE {
            return Err(AutomotiveError::BufferOverflow);
        }
        if self.tail >= TX_QUEUE_SIZE {
            self.tail = 0;
        }
        self.frames.push(frame);
        self.count += 1;
        self.tail += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<Frame> {
        if self.count == 0 {
            return None;
        }
        if self.head >= TX_QUEUE_SIZE {
            self.head = 0;
        }
        let frame = self.frames.remove(self.head);
        self.count -= 1;
        self.head += 1;
        Some(frame)
    }
}

impl RxQueue {
    fn new() -> Self {
        Self {
            frames: Vec::with_capacity(RX_QUEUE_SIZE),
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push(&mut self, frame: Frame) -> Result<()> {
        if self.count >= RX_QUEUE_SIZE {
            return Err(AutomotiveError::BufferOverflow);
        }
        if self.tail >= RX_QUEUE_SIZE {
            self.tail = 0;
        }
        self.frames.push(frame);
        self.count += 1;
        self.tail += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<Frame> {
        if self.count == 0 {
            return None;
        }
        if self.head >= RX_QUEUE_SIZE {
            self.head = 0;
        }
        let frame = self.frames.remove(self.head);
        self.count -= 1;
        self.head += 1;
        Some(frame)
    }
}

impl<P: Port> Can<P> {
    /// Creates a new CAN instance with the given port
    pub fn with_port(config: CanConfig, port: P) -> Self {
        Self {
            config,
            port,
            is_open: false,
            tx_queue: TxQueue::new(),
            rx_queue: RxQueue::new(),
            error_counters: (0, 0),
        }
    }

    /// Configure CAN controller with standard bitrate profile
    pub fn with_bitrate(port: P, bitrate: CanBitrate, options: CanOptions) -> Self {
        let (rate, sample_point, sjw) = match bitrate {
            CanBitrate::Rate1M => (1_000_000, 0.75, 1),
            CanBitrate::Rate500K => (500_000, 0.75, 1),
            CanBitrate::Rate250K => (250_000, 0.75, 1),
            CanBitrate::Rate125K => (125_000, 0.75, 1),
            CanBitrate::Rate100K => (100_000, 0.75, 1),
            CanBitrate::Rate50K => (50_000, 0.75, 1),
            CanBitrate::Rate20K => (20_000, 0.75, 1),
            CanBitrate::Rate10K => (10_000, 0.75, 1),
            CanBitrate::Custom(rate, sp, s) => (rate, sp, s),
        };

        let config = CanConfig {
            bitrate: rate,
            sample_point,
            sjw,
            options,
        };

        Self::with_port(config, port)
    }

    /// Get current error counters (TEC, REC)
    pub fn get_error_counters(&self) -> (u8, u8) {
        self.error_counters
    }

    /// Get number of frames pending in TX queue
    pub fn tx_pending(&self) -> usize {
        self.tx_queue.count
    }

    /// Get number of frames pending in RX queue
    pub fn rx_pending(&self) -> usize {
        self.rx_queue.count
    }

    /// Get space available in TX queue
    pub fn tx_space(&self) -> usize {
        TX_QUEUE_SIZE - self.tx_queue.count
    }

    /// Get space available in RX queue  
    pub fn rx_space(&self) -> usize {
        RX_QUEUE_SIZE - self.rx_queue.count
    }
}

impl<P: Port> PhysicalLayer for Can<P> {
    type Config = CanConfig;

    fn new(_config: Self::Config) -> Result<Self> {
        Err(AutomotiveError::NotInitialized) // Requires platform-specific port
    }

    fn open(&mut self) -> Result<()> {
        if self.is_open {
            return Ok(());
        }

        self.config.validate()?;
        self.is_open = true;
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        self.is_open = false;
        Ok(())
    }

    fn send_frame(&mut self, frame: &Frame) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        if frame.is_fd {
            return Err(AutomotiveError::InvalidParameter);
        }

        // Queue frame for transmission
        self.tx_queue.push(frame.clone())?;

        // Try to send frame via port
        if let Some(frame) = self.tx_queue.pop() {
            self.port.send(&frame)?;
        }

        Ok(())
    }

    fn receive_frame(&mut self) -> Result<Frame> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        // Check RX queue first
        if let Some(frame) = self.rx_queue.pop() {
            return Ok(frame);
        }

        // Try to receive from port
        let frame = self.port.receive()?;
        if frame.is_fd {
            return Err(AutomotiveError::InvalidParameter);
        }

        Ok(frame)
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        self.port.set_timeout(timeout_ms)
    }
}
