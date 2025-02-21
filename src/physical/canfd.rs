use super::PhysicalLayer;
use crate::error::{AutomotiveError, Result};
use crate::types::{Config, Frame, Port};
use bitflags::bitflags;
use std::sync::Arc;

/// CANFD configuration
#[derive(Debug, Clone)]
pub struct CanFdConfig {
    pub nominal_bitrate: u32,
    pub data_bitrate: u32,
    pub nominal_sample_point: f32,
    pub data_sample_point: f32,
    pub nominal_sjw: u8,
    pub data_sjw: u8,
    pub options: CanFdOptions,
}

bitflags! {
    /// CANFD controller options
    #[derive(Debug, Clone)]
    pub struct CanFdOptions: u16 {
        const NONE = 0;
        const REJECT_REMOTE = 1 << 0;
        const HARD_RESET = 1 << 1;
        const RECV_ERRORS = 1 << 2;
        const OPEN_DRAIN = 1 << 3;
        const RECORD_TX_EVENTS = 1 << 4;
        const REJECT_OVERFLOW = 1 << 5;
        const ISO_MODE = 1 << 6;  // ISO CAN FD mode (vs non-ISO)
        const BRS_ENABLE = 1 << 7; // Enable bit rate switching
    }
}

/// Standard CANFD bit rate profiles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CanFdBitrate {
    Rate500k2m, // 500kbit/sec nominal, 2Mbit/sec data rate, 75% sample point
    Rate500k4m, // 500kbit/sec nominal, 4Mbit/sec data rate, 75% sample point
    Rate500k8m, // 500kbit/sec nominal, 8Mbit/sec data rate, 75% sample point
    Rate1m4m,   // 1Mbit/sec nominal, 4Mbit/sec data rate, 75% sample point
    Rate1m8m,   // 1Mbit/sec nominal, 8Mbit/sec data rate, 75% sample point
    Rate250k1m, // 250kbit/sec nominal, 1Mbit/sec data rate, 75% sample point
    Rate250k2m, // 250kbit/sec nominal, 2Mbit/sec data rate, 75% sample point
    Rate250k4m, // 250kbit/sec nominal, 4Mbit/sec data rate, 75% sample point
    Custom(u32, u32, f32, f32, u8, u8), // Custom nominal and data bitrates, sample points, and SJWs
}

const TX_QUEUE_SIZE: usize = 32;
const RX_QUEUE_SIZE: usize = 128;
const TX_EVENT_QUEUE_SIZE: usize = 32;

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

#[derive(Debug)]
struct TxEventQueue {
    events: Vec<TxEvent>,
    head: usize,
    tail: usize,
    count: usize,
}

#[derive(Debug, Clone)]
pub struct TxEvent {
    pub timestamp: u32,
    pub frame: Arc<Frame>,
    pub sequence: u32,
}

impl Config for CanFdConfig {
    fn validate(&self) -> Result<()> {
        if self.nominal_bitrate == 0 || self.data_bitrate == 0 {
            return Err(AutomotiveError::InvalidParameter);
        }
        if self.nominal_sample_point <= 0.0
            || self.nominal_sample_point >= 1.0
            || self.data_sample_point <= 0.0
            || self.data_sample_point >= 1.0
        {
            return Err(AutomotiveError::InvalidParameter);
        }
        if self.nominal_sjw == 0 || self.data_sjw == 0 {
            return Err(AutomotiveError::InvalidParameter);
        }
        Ok(())
    }
}

/// CANFD implementation
pub struct CanFd<P: Port> {
    config: CanFdConfig,
    port: P,
    is_open: bool,
    tx_queue: TxQueue,
    rx_queue: RxQueue,
    tx_events: TxEventQueue,
    error_counters: (u8, u8), // (TEC, REC)
    sequence: u32,
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

impl TxEventQueue {
    fn new() -> Self {
        Self {
            events: Vec::with_capacity(TX_EVENT_QUEUE_SIZE),
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push(&mut self, event: TxEvent) -> Result<()> {
        if self.count >= TX_EVENT_QUEUE_SIZE {
            return Err(AutomotiveError::BufferOverflow);
        }
        if self.tail >= TX_EVENT_QUEUE_SIZE {
            self.tail = 0;
        }
        self.events.push(event);
        self.count += 1;
        self.tail += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<TxEvent> {
        if self.count == 0 {
            return None;
        }
        if self.head >= TX_EVENT_QUEUE_SIZE {
            self.head = 0;
        }
        let event = self.events.remove(self.head);
        self.count -= 1;
        self.head += 1;
        Some(event)
    }
}

impl<P: Port> CanFd<P> {
    /// Creates a new CANFD instance with the given port
    pub fn with_port(config: CanFdConfig, port: P) -> Self {
        Self {
            config,
            port,
            is_open: false,
            tx_queue: TxQueue::new(),
            rx_queue: RxQueue::new(),
            tx_events: TxEventQueue::new(),
            error_counters: (0, 0),
            sequence: 0,
        }
    }

    /// Configure CANFD controller with standard bitrate profile
    pub fn with_bitrate(port: P, bitrate: CanFdBitrate, options: CanFdOptions) -> Self {
        let (nominal_rate, data_rate, nominal_sp, data_sp, nominal_sjw, data_sjw) = match bitrate {
            CanFdBitrate::Rate500k2m => (500_000, 2_000_000, 0.75, 0.75, 1, 1),
            CanFdBitrate::Rate500k4m => (500_000, 4_000_000, 0.75, 0.75, 1, 1),
            CanFdBitrate::Rate500k8m => (500_000, 8_000_000, 0.75, 0.75, 1, 1),
            CanFdBitrate::Rate1m4m => (1_000_000, 4_000_000, 0.75, 0.75, 1, 1),
            CanFdBitrate::Rate1m8m => (1_000_000, 8_000_000, 0.75, 0.75, 1, 1),
            CanFdBitrate::Rate250k1m => (250_000, 1_000_000, 0.75, 0.75, 1, 1),
            CanFdBitrate::Rate250k2m => (250_000, 2_000_000, 0.75, 0.75, 1, 1),
            CanFdBitrate::Rate250k4m => (250_000, 4_000_000, 0.75, 0.75, 1, 1),
            CanFdBitrate::Custom(
                nominal_bitrate,
                data_bitrate,
                nominal_sample_point,
                data_sample_point,
                nominal_sjw,
                data_sjw,
            ) => (
                nominal_bitrate,
                data_bitrate,
                nominal_sample_point,
                data_sample_point,
                nominal_sjw,
                data_sjw,
            ),
        };

        let config = CanFdConfig {
            nominal_bitrate: nominal_rate,
            data_bitrate: data_rate,
            nominal_sample_point: nominal_sp,
            data_sample_point: data_sp,
            nominal_sjw,
            data_sjw,
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

    /// Get number of events pending in TX event queue
    pub fn tx_events_pending(&self) -> usize {
        self.tx_events.count
    }
}

impl<P: Port> PhysicalLayer for CanFd<P> {
    type Config = CanFdConfig;

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

        // Queue frame for transmission
        self.tx_queue.push(frame.clone())?;

        // Try to send frame via port
        if let Some(frame) = self.tx_queue.pop() {
            // Record transmission event if enabled
            if self.config.options.contains(CanFdOptions::RECORD_TX_EVENTS) {
                let event = TxEvent {
                    timestamp: 0, // Will be filled by hardware
                    frame: Arc::new(frame.clone()),
                    sequence: self.sequence,
                };
                self.sequence = self.sequence.wrapping_add(1);
                self.tx_events.push(event)?;
            }

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

        // Handle remote frames if configured to reject them
        if frame.is_extended && self.config.options.contains(CanFdOptions::REJECT_REMOTE) {
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
