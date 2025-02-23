use std::collections::HashMap;
use std::time::Duration;

use super::isobus_diagnostic::{DiagnosticTroubleCode, ISOBUSDiagnosticProtocol};
use super::TransportLayer;
use crate::error::{AutomotiveError, Result};
use crate::types::{Config, Frame};

// ISOBUS Protocol Constants
const ISOBUS_PROTOCOL_VERSION: u8 = 0x03;

// ISOBUS PGNs (Parameter Group Numbers)
const PGN_ADDRESS_CLAIM: u32 = 0x00EE00;
const PGN_REQUEST: u32 = 0x00EA00;
const PGN_TRANSPORT_PROTOCOL_CONNECTION: u32 = 0x00EC00;
const PGN_TRANSPORT_PROTOCOL_DATA: u32 = 0x00EB00;
const PGN_DIAGNOSTIC_MESSAGE: u32 = 0x00FECA;

// Transport Protocol Control Bytes
const TP_CM_RTS: u8 = 0x10; // Request to Send
const TP_CM_CTS: u8 = 0x11; // Clear to Send
const TP_CM_EndOfMsgACK: u8 = 0x13; // End of Message Acknowledgment
const TP_CM_BAM: u8 = 0x20; // Broadcast Announce Message
const TP_CM_ABORT: u8 = 0xFF; // Connection Abort

// Timeouts (in milliseconds)
const T1_TIMEOUT: u32 = 750; // Time between CTS and first data packet
const T2_TIMEOUT: u32 = 1250; // Time between consecutive data packets
const T3_TIMEOUT: u32 = 1250; // Time between last data packet and EndOfMsgACK
const T4_TIMEOUT: u32 = 1050; // Time waiting for CTS

#[derive(Debug, Clone)]
pub struct ISOBUSConfig {
    pub source_address: u8,
    pub name: u64, // ISO NAME (64-bit identifier)
    pub preferred_address: u8,
    pub function_instance: u8,
    pub ecu_instance: u8,
    pub manufacturer_code: u16,
    pub identity_number: u32,
    pub timeout_ms: u32,
}

impl Config for ISOBUSConfig {
    fn validate(&self) -> Result<()> {
        if self.source_address > 0xFE {
            return Err(AutomotiveError::InvalidParameter);
        }
        if self.preferred_address > 0xFE {
            return Err(AutomotiveError::InvalidParameter);
        }
        Ok(())
    }
}

impl Default for ISOBUSConfig {
    fn default() -> Self {
        Self {
            source_address: 0x80, // Default source address
            name: 0,              // Must be set by user
            preferred_address: 0x80,
            function_instance: 0,
            ecu_instance: 0,
            manufacturer_code: 0,
            identity_number: 0,
            timeout_ms: 1000,
        }
    }
}

// Transport Protocol Session State
#[derive(Debug)]
enum TPSessionState {
    Idle,
    WaitingForCTS,
    SendingData,
    WaitingForEndOfMsgACK,
    ReceivingData,
}

// Transport Protocol Session
#[derive(Debug)]
struct TPSession {
    state: TPSessionState,
    total_size: u16,
    total_packets: u8,
    next_packet: u8,
    data: Vec<u8>,
    source_address: u8,
    destination_address: u8,
    pgn: u32,
    last_timestamp: u64,
}

pub struct ISOBUS {
    config: ISOBUSConfig,
    is_open: bool,
    address_claimed: bool,
    tp_sessions: HashMap<u8, TPSession>, // Key is source address
    rx_buffer: Vec<u8>,
    diagnostic_protocol: ISOBUSDiagnosticProtocol,
}

impl ISOBUS {
    fn claim_address(&mut self) -> Result<()> {
        // Create NAME field
        let name_bytes = self.config.name.to_be_bytes();

        // Send address claim message
        let mut frame = Frame {
            id: ((PGN_ADDRESS_CLAIM as u32) << 8) | (self.config.source_address as u32),
            data: name_bytes.to_vec(),
            timestamp: 0,
            is_extended: true,
            is_fd: false,
        };

        self.write_frame(&frame)?;

        // Wait for potential address conflicts
        std::thread::sleep(Duration::from_millis(250));

        self.address_claimed = true;
        Ok(())
    }

    fn handle_transport_protocol(&mut self, frame: &Frame) -> Result<()> {
        let source_address = (frame.id & 0xFF) as u8;
        let pgn = (frame.id >> 8) as u32;

        match pgn {
            PGN_TRANSPORT_PROTOCOL_CONNECTION => {
                self.handle_tp_connection(source_address, &frame.data)?;
            }
            PGN_TRANSPORT_PROTOCOL_DATA => {
                self.handle_tp_data(source_address, &frame.data)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_tp_connection(&mut self, source_address: u8, data: &[u8]) -> Result<()> {
        let control_byte = data[0];

        match control_byte {
            TP_CM_RTS => {
                let size = ((data[2] as u16) << 8) | (data[1] as u16);
                let total_packets = data[3];
                let pgn = ((data[7] as u32) << 16) | ((data[6] as u32) << 8) | (data[5] as u32);

                let session = TPSession {
                    state: TPSessionState::ReceivingData,
                    total_size: size,
                    total_packets,
                    next_packet: 1,
                    data: Vec::with_capacity(size as usize),
                    source_address,
                    destination_address: self.config.source_address,
                    pgn,
                    last_timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                };

                self.tp_sessions.insert(source_address, session);

                // Send CTS
                let mut cts_frame = Frame {
                    id: ((PGN_TRANSPORT_PROTOCOL_CONNECTION as u32) << 8)
                        | (self.config.source_address as u32),
                    data: vec![
                        TP_CM_CTS,
                        total_packets, // Number of packets that can be sent
                        1,             // Next packet number
                        0xFF,
                        0xFF,    // Reserved
                        data[5], // PGN
                        data[6],
                        data[7],
                    ],
                    timestamp: 0,
                    is_extended: true,
                    is_fd: false,
                };

                self.write_frame(&cts_frame)?;
            }
            TP_CM_CTS => {
                if let Some(session) = self.tp_sessions.get_mut(&source_address) {
                    session.state = TPSessionState::SendingData;
                    session.next_packet = data[2];
                }
            }
            TP_CM_EndOfMsgACK => {
                self.tp_sessions.remove(&source_address);
            }
            TP_CM_ABORT => {
                self.tp_sessions.remove(&source_address);
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_tp_data(&mut self, source_address: u8, data: &[u8]) -> Result<()> {
        if let Some(session) = self.tp_sessions.get_mut(&source_address) {
            let sequence = data[0];
            if sequence == session.next_packet {
                session.data.extend_from_slice(&data[1..]);
                session.next_packet += 1;
                session.last_timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                if session.next_packet > session.total_packets {
                    // Send End of Message ACK
                    let mut ack_frame = Frame {
                        id: ((PGN_TRANSPORT_PROTOCOL_CONNECTION as u32) << 8)
                            | (self.config.source_address as u32),
                        data: vec![
                            TP_CM_EndOfMsgACK,
                            (session.total_size & 0xFF) as u8,
                            ((session.total_size >> 8) & 0xFF) as u8,
                            session.total_packets,
                            0xFF,
                            (session.pgn & 0xFF) as u8,
                            ((session.pgn >> 8) & 0xFF) as u8,
                            ((session.pgn >> 16) & 0xFF) as u8,
                        ],
                        timestamp: 0,
                        is_extended: true,
                        is_fd: false,
                    };

                    self.write_frame(&ack_frame)?;
                    self.tp_sessions.remove(&source_address);
                }
            }
        }

        Ok(())
    }

    fn handle_diagnostic_message(&mut self, frame: &Frame) -> Result<()> {
        if let Some(response) = self.diagnostic_protocol.process_message(frame)? {
            self.write_frame(&response)?;
        }
        Ok(())
    }

    fn update_diagnostic_protocol(&mut self) -> Result<()> {
        if let Some(frame) = self.diagnostic_protocol.update()? {
            self.write_frame(&frame)?;
        }
        Ok(())
    }

    pub fn add_dtc(&mut self, dtc: DiagnosticTroubleCode) {
        self.diagnostic_protocol.add_dtc(dtc);
    }

    pub fn clear_dtcs(&mut self) {
        self.diagnostic_protocol.clear_active_dtcs();
        self.diagnostic_protocol.clear_inactive_dtcs();
    }

    pub fn get_active_dtcs(&self) -> Vec<&DiagnosticTroubleCode> {
        self.diagnostic_protocol.get_active_dtcs()
    }

    pub fn get_inactive_dtcs(&self) -> Vec<&DiagnosticTroubleCode> {
        self.diagnostic_protocol.get_inactive_dtcs()
    }
}

impl TransportLayer for ISOBUS {
    type Config = ISOBUSConfig;

    fn new(config: Self::Config) -> Result<Self> {
        Ok(Self {
            config,
            is_open: false,
            address_claimed: false,
            tp_sessions: HashMap::new(),
            rx_buffer: Vec::new(),
            diagnostic_protocol: ISOBUSDiagnosticProtocol::new(),
        })
    }

    fn open(&mut self) -> Result<()> {
        if self.is_open {
            return Ok(());
        }

        // Validate configuration
        self.config.validate()?;

        // Claim address on the bus
        self.claim_address()?;

        self.is_open = true;
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        self.is_open = false;
        self.address_claimed = false;
        self.tp_sessions.clear();
        Ok(())
    }

    fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        if !self.address_claimed {
            return Err(AutomotiveError::NotInitialized);
        }

        // Check if message needs transport protocol
        if frame.data.len() > 8 {
            // Implement transport protocol for large messages
            let total_size = frame.data.len() as u16;
            let total_packets = ((total_size + 6) / 7) as u8;
            let pgn = (frame.id >> 8) as u32;

            // Send RTS
            let mut rts_frame = Frame {
                id: ((PGN_TRANSPORT_PROTOCOL_CONNECTION as u32) << 8)
                    | (self.config.source_address as u32),
                data: vec![
                    TP_CM_RTS,
                    (total_size & 0xFF) as u8,
                    ((total_size >> 8) & 0xFF) as u8,
                    total_packets,
                    0xFF,
                    (pgn & 0xFF) as u8,
                    ((pgn >> 8) & 0xFF) as u8,
                    ((pgn >> 16) & 0xFF) as u8,
                ],
                timestamp: 0,
                is_extended: true,
                is_fd: false,
            };

            self.write_frame(&rts_frame)?;

            let session = TPSession {
                state: TPSessionState::WaitingForCTS,
                total_size,
                total_packets,
                next_packet: 1,
                data: frame.data.clone(),
                source_address: self.config.source_address,
                destination_address: (frame.id & 0xFF) as u8,
                pgn,
                last_timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            };

            self.tp_sessions.insert(self.config.source_address, session);
        } else {
            // Direct transmission for small messages
            // Implement CAN frame transmission here
            // This would interface with the actual CAN hardware
        }

        Ok(())
    }

    fn read_frame(&mut self) -> Result<Frame> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        if !self.address_claimed {
            return Err(AutomotiveError::NotInitialized);
        }

        // Update diagnostic protocol
        self.update_diagnostic_protocol()?;

        // Implement CAN frame reception here
        // This would interface with the actual CAN hardware

        Err(AutomotiveError::PortError(
            "CAN hardware interface not implemented".to_string(),
        ))
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        self.config.timeout_ms = timeout_ms;
        Ok(())
    }
}
