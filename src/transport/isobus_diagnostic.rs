use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{AutomotiveError, Result};
use crate::types::Frame;

// ISOBUS Diagnostic Message Parameter Group Numbers (PGNs)
const PGN_DM1: u32 = 0x00FECA; // DM1: Active Diagnostic Trouble Codes (DTCs)
const PGN_DM2: u32 = 0x00FECB; // DM2: Previously Active Diagnostic Trouble Codes
const PGN_DM3: u32 = 0x00FECC; // DM3: Diagnostic Data Clear/Reset for All DTCs
const PGN_DM11: u32 = 0x00FED4; // DM11: Diagnostic Data Clear/Reset for Active DTCs Only
const PGN_DM13: u32 = 0x00FED6; // DM13: Stop/Start Broadcast of DM1 Message
const PGN_DM22: u32 = 0x00FEE3; // DM22: Individual Clear/Reset of Specific Active and Previously Active DTCs

// Diagnostic Message Timing Parameters
const DM1_BROADCAST_INTERVAL_MS: u64 = 1000; // Broadcast interval for DM1 messages (1 second)

// Malfunction Indicator Lamp (MIL) Status Values
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LampStatus {
    Off = 0,       // Lamp is turned off
    On = 1,        // Lamp is continuously on
    SlowFlash = 2, // Lamp is flashing slowly (1 Hz)
    FastFlash = 3, // Lamp is flashing rapidly (2 Hz)
}

/// Represents a single Diagnostic Trouble Code (DTC) in the ISOBUS system
#[derive(Debug, Clone)]
pub struct DiagnosticTroubleCode {
    spn: u32,                // Suspect Parameter Number (19-bit identifier)
    fmi: u8,                 // Failure Mode Identifier (5-bit value)
    occurrence_count: u8,    // Number of occurrences of this DTC (7-bit counter)
    lamp_status: LampStatus, // Status of the Malfunction Indicator Lamp
    active: bool,            // Indicates if the DTC is currently active
}

impl DiagnosticTroubleCode {
    /// Creates a new Diagnostic Trouble Code with default values
    pub fn new(spn: u32, fmi: u8) -> Self {
        Self {
            spn,
            fmi,
            occurrence_count: 1,
            lamp_status: LampStatus::Off,
            active: true,
        }
    }

    /// Converts the DTC into a byte array format according to SAE J1939-73
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4);

        // Pack SPN (19 bits), FMI (5 bits), and occurrence count (7 bits)
        // into a 32-bit value according to the protocol specification
        let spn_bytes = ((self.spn & 0x7FFFF) << 5) as u32;
        let fmi_byte = (self.fmi & 0x1F) as u32;
        let count_byte = ((self.occurrence_count & 0x7F) as u32) << 24;

        let combined = spn_bytes | fmi_byte | count_byte;
        bytes.extend_from_slice(&combined.to_be_bytes());

        bytes
    }

    /// Creates a DTC from a byte array according to SAE J1939-73 format
    fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(AutomotiveError::InvalidData);
        }

        let combined = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);

        // Extract SPN (19 bits), FMI (5 bits), and occurrence count (7 bits)
        let spn = (combined >> 5) & 0x7FFFF;
        let fmi = (combined & 0x1F) as u8;
        let occurrence_count = (combined >> 24) as u8;

        Ok(Self {
            spn,
            fmi,
            occurrence_count,
            lamp_status: LampStatus::Off,
            active: true,
        })
    }
}

/// Implements the ISOBUS Diagnostic Protocol according to SAE J1939-73
pub struct ISOBUSDiagnosticProtocol {
    active_dtcs: HashMap<(u32, u8), DiagnosticTroubleCode>, // Currently active DTCs, keyed by (SPN, FMI)
    inactive_dtcs: HashMap<(u32, u8), DiagnosticTroubleCode>, // Previously active DTCs, keyed by (SPN, FMI)
    last_dm1_broadcast: u64,                                  // Timestamp of last DM1 broadcast
    broadcast_enabled: bool,                                  // Controls DM1 message broadcasting
}

impl ISOBUSDiagnosticProtocol {
    /// Creates a new instance of the ISOBUS Diagnostic Protocol handler
    pub fn new() -> Self {
        Self {
            active_dtcs: HashMap::new(),
            inactive_dtcs: HashMap::new(),
            last_dm1_broadcast: 0,
            broadcast_enabled: true,
        }
    }

    /// Adds or updates a Diagnostic Trouble Code in the appropriate storage
    /// If the DTC already exists, its occurrence count is incremented
    pub fn add_dtc(&mut self, dtc: DiagnosticTroubleCode) {
        let key = (dtc.spn, dtc.fmi);
        if dtc.active {
            if let Some(existing) = self.active_dtcs.get_mut(&key) {
                existing.occurrence_count = existing.occurrence_count.saturating_add(1);
            } else {
                self.active_dtcs.insert(key, dtc);
            }
        } else {
            if let Some(existing) = self.inactive_dtcs.get_mut(&key) {
                existing.occurrence_count = existing.occurrence_count.saturating_add(1);
            } else {
                self.inactive_dtcs.insert(key, dtc);
            }
        }
    }

    /// Clears all active DTCs (DM11 functionality)
    pub fn clear_active_dtcs(&mut self) {
        self.active_dtcs.clear();
    }

    /// Clears all previously active DTCs
    pub fn clear_inactive_dtcs(&mut self) {
        self.inactive_dtcs.clear();
    }

    /// Clears a specific DTC identified by its SPN and FMI (DM22 functionality)
    pub fn clear_dtc(&mut self, spn: u32, fmi: u8) {
        let key = (spn, fmi);
        self.active_dtcs.remove(&key);
        self.inactive_dtcs.remove(&key);
    }

    /// Controls the broadcasting of DM1 messages (DM13 functionality)
    pub fn set_broadcast_enabled(&mut self, enabled: bool) {
        self.broadcast_enabled = enabled;
    }

    /// Processes incoming diagnostic messages according to their PGN
    pub fn process_message(&mut self, frame: &Frame) -> Result<Option<Frame>> {
        let pgn = (frame.id >> 8) as u32;

        match pgn {
            PGN_DM3 => {
                // DM3: Clear all active and previously active DTCs
                self.clear_active_dtcs();
                self.clear_inactive_dtcs();
                Ok(None)
            }
            PGN_DM11 => {
                // DM11: Clear only active DTCs
                self.clear_active_dtcs();
                Ok(None)
            }
            PGN_DM13 => {
                // DM13: Control broadcast of DM1 messages
                if frame.data.len() >= 1 {
                    self.broadcast_enabled = frame.data[0] != 0;
                }
                Ok(None)
            }
            PGN_DM22 => {
                // DM22: Clear specific DTC
                if frame.data.len() >= 4 {
                    if let Ok(dtc) = DiagnosticTroubleCode::from_bytes(&frame.data) {
                        self.clear_dtc(dtc.spn, dtc.fmi);
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Updates the diagnostic state and generates DM1 broadcast messages if needed
    pub fn update(&mut self) -> Result<Option<Frame>> {
        if !self.broadcast_enabled {
            return Ok(None);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Check if it's time to broadcast DM1 message
        if now - self.last_dm1_broadcast >= DM1_BROADCAST_INTERVAL_MS {
            self.last_dm1_broadcast = now;

            if !self.active_dtcs.is_empty() {
                // Create DM1 broadcast message containing active DTCs
                let mut data = Vec::new();

                // First two bytes contain lamp status information
                data.extend_from_slice(&[0x00, 0x00]);

                // Add each active DTC to the message
                for dtc in self.active_dtcs.values() {
                    data.extend(dtc.to_bytes());
                }

                let frame = Frame {
                    id: (PGN_DM1 << 8) as u32,
                    data,
                    timestamp: now as u64,
                    is_extended: true,
                    is_fd: false,
                };

                return Ok(Some(frame));
            }
        }

        Ok(None)
    }

    /// Returns a vector of references to all active DTCs
    pub fn get_active_dtcs(&self) -> Vec<&DiagnosticTroubleCode> {
        self.active_dtcs.values().collect()
    }

    /// Returns a vector of references to all previously active DTCs
    pub fn get_inactive_dtcs(&self) -> Vec<&DiagnosticTroubleCode> {
        self.inactive_dtcs.values().collect()
    }
}
