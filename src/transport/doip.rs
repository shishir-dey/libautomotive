use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use super::TransportLayer;
use crate::error::{AutomotiveError, Result};
use crate::physical::PhysicalLayer;
use crate::types::{Config, Frame};

// DoIP protocol version
const DOIP_PROTOCOL_VERSION: u8 = 0x02;

// DoIP message types
const DOIP_VEHICLE_IDENTIFICATION_REQUEST: u16 = 0x0001;
const DOIP_VEHICLE_IDENTIFICATION_RESPONSE: u16 = 0x0002;
const DOIP_ROUTING_ACTIVATION_REQUEST: u16 = 0x0005;
const DOIP_ROUTING_ACTIVATION_RESPONSE: u16 = 0x0006;
const DOIP_DIAGNOSTIC_MESSAGE: u16 = 0x8001;
const DOIP_DIAGNOSTIC_MESSAGE_POSITIVE_ACK: u16 = 0x8002;
const DOIP_DIAGNOSTIC_MESSAGE_NEGATIVE_ACK: u16 = 0x8003;

// DoIP header structure
#[derive(Debug, Clone)]
struct DoIPHeader {
    protocol_version: u8,
    inverse_version: u8,
    payload_type: u16,
    payload_length: u32,
}

impl DoIPHeader {
    fn new(payload_type: u16, payload_length: u32) -> Self {
        Self {
            protocol_version: DOIP_PROTOCOL_VERSION,
            inverse_version: !DOIP_PROTOCOL_VERSION,
            payload_type,
            payload_length,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);
        bytes.push(self.protocol_version);
        bytes.push(self.inverse_version);
        bytes.extend_from_slice(&self.payload_type.to_be_bytes());
        bytes.extend_from_slice(&self.payload_length.to_be_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            return Err(AutomotiveError::InvalidData);
        }

        let protocol_version = bytes[0];
        let inverse_version = bytes[1];
        let payload_type = u16::from_be_bytes([bytes[2], bytes[3]]);
        let payload_length = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        if protocol_version != DOIP_PROTOCOL_VERSION || inverse_version != !DOIP_PROTOCOL_VERSION {
            return Err(AutomotiveError::InvalidData);
        }

        Ok(Self {
            protocol_version,
            inverse_version,
            payload_type,
            payload_length,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DoIPConfig {
    pub host: String,
    pub port: u16,
    pub target_address: u16,
    pub source_address: u16,
    pub timeout_ms: u32,
    pub tcp_connection_timeout_ms: u32,
    pub response_timeout_ms: u32,
}

impl Config for DoIPConfig {
    fn validate(&self) -> Result<()> {
        if self.port == 0 {
            return Err(AutomotiveError::InvalidParameter);
        }
        if self.target_address == 0 || self.source_address == 0 {
            return Err(AutomotiveError::InvalidParameter);
        }
        Ok(())
    }
}

impl Default for DoIPConfig {
    fn default() -> Self {
        Self {
            host: String::from("localhost"),
            port: 13400,            // Default DoIP port
            target_address: 0x0E80, // Default diagnostic address
            source_address: 0x0E00, // Default tester address
            timeout_ms: 5000,
            tcp_connection_timeout_ms: 2000,
            response_timeout_ms: 5000,
        }
    }
}

pub struct DoIP<P: PhysicalLayer> {
    config: DoIPConfig,
    physical: P,
    stream: Option<TcpStream>,
    is_open: bool,
}

impl<P: PhysicalLayer> DoIP<P> {
    /// Creates a new DoIP instance with the given physical layer
    pub fn with_physical(config: DoIPConfig, physical: P) -> Self {
        Self {
            config,
            physical,
            stream: None,
            is_open: false,
        }
    }

    fn activate_routing(&mut self) -> Result<()> {
        let stream = self
            .stream
            .as_mut()
            .ok_or(AutomotiveError::NotInitialized)?;

        // Create routing activation request
        let mut payload = Vec::new();
        payload.extend_from_slice(&self.config.source_address.to_be_bytes());
        payload.push(0x00); // Activation type: Default
        payload.push(0x00); // Reserved
        payload.extend_from_slice(&[0x00, 0x00]); // Reserved

        let header = DoIPHeader::new(DOIP_ROUTING_ACTIVATION_REQUEST, payload.len() as u32);
        let mut message = header.to_bytes();
        message.extend(payload);

        // Send request
        stream
            .write_all(&message)
            .map_err(|_| AutomotiveError::SendFailed)?;

        // Read response
        let mut header_buf = [0u8; 8];
        stream
            .read_exact(&mut header_buf)
            .map_err(|_| AutomotiveError::ReceiveFailed)?;

        let response_header = DoIPHeader::from_bytes(&header_buf)?;
        if response_header.payload_type != DOIP_ROUTING_ACTIVATION_RESPONSE {
            return Err(AutomotiveError::InvalidData);
        }

        let mut response_payload = vec![0u8; response_header.payload_length as usize];
        stream
            .read_exact(&mut response_payload)
            .map_err(|_| AutomotiveError::ReceiveFailed)?;

        // Check response code (first byte of payload)
        if response_payload[0] != 0x10 {
            // 0x10 = Routing activation successful
            return Err(AutomotiveError::ConnectionFailed);
        }

        Ok(())
    }
}

impl<P: PhysicalLayer> TransportLayer for DoIP<P> {
    type Config = DoIPConfig;

    fn new(config: Self::Config) -> Result<Self> {
        Err(AutomotiveError::NotInitialized) // Requires physical layer
    }

    fn open(&mut self) -> Result<()> {
        if self.is_open {
            return Ok(());
        }

        // Validate configuration
        self.config.validate()?;

        // Connect to DoIP server
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let stream = TcpStream::connect(&addr).map_err(|_| AutomotiveError::ConnectionFailed)?;

        stream
            .set_read_timeout(Some(Duration::from_millis(self.config.timeout_ms as u64)))
            .map_err(|_| AutomotiveError::ConnectionFailed)?;
        stream
            .set_write_timeout(Some(Duration::from_millis(self.config.timeout_ms as u64)))
            .map_err(|_| AutomotiveError::ConnectionFailed)?;

        self.stream = Some(stream);
        self.is_open = true;

        // Perform routing activation
        self.activate_routing()?;

        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        self.is_open = false;
        Ok(())
    }

    fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        let stream = self
            .stream
            .as_mut()
            .ok_or(AutomotiveError::NotInitialized)?;

        // Create diagnostic message
        let mut payload = Vec::new();
        payload.extend_from_slice(&self.config.source_address.to_be_bytes());
        payload.extend_from_slice(&self.config.target_address.to_be_bytes());
        payload.extend(&frame.data);

        let header = DoIPHeader::new(DOIP_DIAGNOSTIC_MESSAGE, payload.len() as u32);
        let mut message = header.to_bytes();
        message.extend(payload);

        stream
            .write_all(&message)
            .map_err(|_| AutomotiveError::SendFailed)?;

        // Read acknowledgment
        let mut header_buf = [0u8; 8];
        stream
            .read_exact(&mut header_buf)
            .map_err(|_| AutomotiveError::ReceiveFailed)?;

        let response_header = DoIPHeader::from_bytes(&header_buf)?;
        match response_header.payload_type {
            DOIP_DIAGNOSTIC_MESSAGE_POSITIVE_ACK => Ok(()),
            DOIP_DIAGNOSTIC_MESSAGE_NEGATIVE_ACK => {
                let mut response_payload = vec![0u8; response_header.payload_length as usize];
                stream
                    .read_exact(&mut response_payload)
                    .map_err(|_| AutomotiveError::ReceiveFailed)?;
                Err(AutomotiveError::DoIPError(format!(
                    "NACK received: 0x{:02X}",
                    response_payload[0]
                )))
            }
            _ => Err(AutomotiveError::InvalidData),
        }
    }

    fn read_frame(&mut self) -> Result<Frame> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        let stream = self
            .stream
            .as_mut()
            .ok_or(AutomotiveError::NotInitialized)?;

        // Read header
        let mut header_buf = [0u8; 8];
        stream
            .read_exact(&mut header_buf)
            .map_err(|_| AutomotiveError::ReceiveFailed)?;

        let header = DoIPHeader::from_bytes(&header_buf)?;

        // Read payload
        let mut payload = vec![0u8; header.payload_length as usize];
        stream
            .read_exact(&mut payload)
            .map_err(|_| AutomotiveError::ReceiveFailed)?;

        // Check if it's a diagnostic message
        if header.payload_type != DOIP_DIAGNOSTIC_MESSAGE {
            return Err(AutomotiveError::InvalidData);
        }

        let diagnostic_data = payload[4..].to_vec();

        Ok(Frame {
            id: 0, // DoIP doesn't use CAN IDs
            data: diagnostic_data,
            timestamp: 0, // TODO: Add proper timestamp
            is_extended: false,
            is_fd: false,
        })
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        let stream = self
            .stream
            .as_mut()
            .ok_or(AutomotiveError::NotInitialized)?;
        let timeout = Duration::from_millis(timeout_ms as u64);

        stream
            .set_read_timeout(Some(timeout))
            .map_err(|_| AutomotiveError::DoIPError("Failed to set read timeout".into()))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|_| AutomotiveError::DoIPError("Failed to set write timeout".into()))?;

        self.config.timeout_ms = timeout_ms;
        Ok(())
    }
}
