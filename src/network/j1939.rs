use super::NetworkLayer;
use crate::error::{AutomotiveError, Result};
use crate::physical::PhysicalLayer;
use crate::types::{Address, Config, Frame};

const PGN_ADDRESS_CLAIMED: u32 = 0xEE00;
const PGN_REQUEST: u32 = 0xEA00;
const PGN_CANNOT_CLAIM: u32 = 0xEE00;

/// J1939 message structure
#[derive(Debug, Clone)]
pub struct J1939Message {
    pub address: Address,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// J1939 configuration
#[derive(Debug, Clone)]
pub struct J1939Config {
    pub name: u64, // 64-bit NAME field
    pub preferred_address: u8,
    pub address_range: (u8, u8),
}

impl Config for J1939Config {
    fn validate(&self) -> Result<()> {
        if self.name == 0 {
            return Err(AutomotiveError::InvalidParameter);
        }
        if self.address_range.0 > self.address_range.1 {
            return Err(AutomotiveError::InvalidParameter);
        }
        if self.preferred_address < self.address_range.0
            || self.preferred_address > self.address_range.1
        {
            return Err(AutomotiveError::InvalidParameter);
        }
        Ok(())
    }
}

/// J1939 implementation
pub struct J1939<P: PhysicalLayer> {
    config: J1939Config,
    physical: P,
    current_address: Option<u8>,
    is_open: bool,
}

impl<P: PhysicalLayer> J1939<P> {
    /// Creates a new J1939 instance with the given physical layer
    pub fn with_physical(config: J1939Config, physical: P) -> Self {
        Self {
            config,
            physical,
            current_address: None,
            is_open: false,
        }
    }

    fn build_frame(&self, address: &Address, data: &[u8]) -> Frame {
        let id = ((address.priority as u32) << 26)
            | ((address.pgn as u32) << 8)
            | (self.current_address.unwrap_or(0xFF) as u32);

        Frame {
            id,
            data: data.to_vec(),
            timestamp: 0,
            is_extended: true,
            is_fd: false,
        }
    }

    fn parse_frame(&self, frame: &Frame) -> Result<J1939Message> {
        if !frame.is_extended {
            return Err(AutomotiveError::J1939Error("Not an extended frame".into()));
        }

        let priority = ((frame.id >> 26) & 0x7) as u8;
        let pgn = ((frame.id >> 8) & 0x3FFFF) as u32;
        let source = (frame.id & 0xFF) as u8;
        let destination = if (pgn & 0xFF00) == 0 {
            (pgn & 0xFF) as u8
        } else {
            0xFF
        };

        Ok(J1939Message {
            address: Address {
                priority,
                pgn,
                source,
                destination,
            },
            data: frame.data.clone(),
            timestamp: frame.timestamp,
        })
    }

    fn send_address_claim(&mut self, address: u8) -> Result<()> {
        let mut name_bytes = Vec::with_capacity(8);
        let mut name = self.config.name;
        for _ in 0..8 {
            name_bytes.push((name & 0xFF) as u8);
            name >>= 8;
        }
        name_bytes.reverse();

        let address = Address {
            priority: 6,
            pgn: PGN_ADDRESS_CLAIMED,
            source: address,
            destination: 0xFF,
        };

        self.send(&address, &name_bytes)
    }
}

impl<P: PhysicalLayer> NetworkLayer for J1939<P> {
    type Config = J1939Config;
    type Message = J1939Message;

    fn new(_config: Self::Config) -> Result<Self> {
        Err(AutomotiveError::NotInitialized)
    }

    fn open(&mut self) -> Result<()> {
        if self.is_open {
            return Ok(());
        }

        self.config.validate()?;
        self.physical.open()?;
        self.is_open = true;

        // Try to claim preferred address
        self.claim_address(self.config.preferred_address)?;

        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        if !self.is_open {
            return Ok(());
        }

        self.physical.close()?;
        self.is_open = false;
        self.current_address = None;
        Ok(())
    }

    fn send(&mut self, address: &Address, data: &[u8]) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        if self.current_address.is_none() {
            return Err(AutomotiveError::J1939Error("No address claimed".into()));
        }

        let frame = self.build_frame(address, data);
        self.physical.send_frame(&frame)
    }

    fn receive(&mut self) -> Result<Self::Message> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        let frame = self.physical.receive_frame()?;
        self.parse_frame(&frame)
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        self.physical.set_timeout(timeout_ms)
    }

    fn claim_address(&mut self, address: u8) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        if address < self.config.address_range.0 || address > self.config.address_range.1 {
            return Err(AutomotiveError::InvalidParameter);
        }

        // Send address claim
        self.send_address_claim(address)?;

        // Store current timeout and set temporary timeout for address claiming
        let current_timeout = 1000; // Default timeout
        self.physical.set_timeout(250)?;

        let result = loop {
            match self.receive() {
                Ok(msg)
                    if msg.address.pgn == PGN_ADDRESS_CLAIMED && msg.address.source == address =>
                {
                    // Compare NAME
                    let mut name = 0u64;
                    for &byte in msg.data.iter().take(8) {
                        name = (name << 8) | byte as u64;
                    }

                    if name < self.config.name {
                        break Err(AutomotiveError::J1939Error(
                            "Address claimed by higher priority device".into(),
                        ));
                    }
                }
                Err(AutomotiveError::Timeout) => break Ok(()),
                _ => continue,
            }
        };

        // Restore original timeout
        self.physical.set_timeout(current_timeout)?;

        match result {
            Ok(()) => {
                self.current_address = Some(address);
                Ok(())
            }
            Err(e) => {
                // Send cannot claim address message
                let cannot_claim = Address {
                    priority: 6,
                    pgn: PGN_CANNOT_CLAIM,
                    source: 0xFE,
                    destination: 0xFF,
                };
                let _ = self.send(&cannot_claim, &[]);
                Err(e)
            }
        }
    }

    fn get_address(&self) -> Result<u8> {
        self.current_address
            .ok_or_else(|| AutomotiveError::J1939Error("No address claimed".into()))
    }
}
