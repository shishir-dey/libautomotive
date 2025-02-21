use super::ApplicationLayer;
use crate::error::{AutomotiveError, Result};
use crate::transport::TransportLayer;
use crate::types::Config;

// UDS Service IDs
pub const SID_DIAGNOSTIC_SESSION_CONTROL: u8 = 0x10;
pub const SID_ECU_RESET: u8 = 0x11;
pub const SID_SECURITY_ACCESS: u8 = 0x27;
pub const SID_TESTER_PRESENT: u8 = 0x3E;
pub const SID_READ_DATA_BY_ID: u8 = 0x22;
pub const SID_WRITE_DATA_BY_ID: u8 = 0x2E;
pub const SID_CLEAR_DIAGNOSTIC_INFO: u8 = 0x14;
pub const SID_READ_DTC: u8 = 0x19;

// Additional UDS Service IDs
pub const SID_COMMUNICATION_CONTROL: u8 = 0x28;
pub const SID_AUTHENTICATION: u8 = 0x29;
pub const SID_READ_DATA_BY_PERIODIC_ID: u8 = 0x2A;
pub const SID_DYNAMICALLY_DEFINE_DATA_ID: u8 = 0x2C;
pub const SID_READ_MEMORY_BY_ADDRESS: u8 = 0x23;
pub const SID_WRITE_MEMORY_BY_ADDRESS: u8 = 0x3D;
pub const SID_READ_SCALING_DATA_BY_ID: u8 = 0x24;
pub const SID_INPUT_OUTPUT_CONTROL_BY_ID: u8 = 0x2F;
pub const SID_ROUTINE_CONTROL: u8 = 0x31;
pub const SID_REQUEST_DOWNLOAD: u8 = 0x34;
pub const SID_REQUEST_UPLOAD: u8 = 0x35;
pub const SID_TRANSFER_DATA: u8 = 0x36;
pub const SID_REQUEST_TRANSFER_EXIT: u8 = 0x37;

// UDS Response Type
#[derive(Debug, Clone, PartialEq)]
pub enum UdsResponseType {
    Positive,
    Negative(u8), // NRC
}

// UDS Session Type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UdsSessionType {
    Default = 0x01,
    Programming = 0x02,
    Extended = 0x03,
    SafetySystem = 0x04,
}

// UDS Reset Type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UdsResetType {
    HardReset = 0x01,
    KeyOffOnReset = 0x02,
    SoftReset = 0x03,
    EnableRapidPowerShutdown = 0x04,
    DisableRapidPowerShutdown = 0x05,
}

// UDS Negative Response Codes
pub const NRC_GENERAL_REJECT: u8 = 0x10;
pub const NRC_SERVICE_NOT_SUPPORTED: u8 = 0x11;
pub const NRC_SUB_FUNCTION_NOT_SUPPORTED: u8 = 0x12;
pub const NRC_INCORRECT_MESSAGE_LENGTH: u8 = 0x13;
pub const NRC_CONDITIONS_NOT_CORRECT: u8 = 0x22;
pub const NRC_REQUEST_SEQUENCE_ERROR: u8 = 0x24;
pub const NRC_REQUEST_OUT_OF_RANGE: u8 = 0x31;
pub const NRC_SECURITY_ACCESS_DENIED: u8 = 0x33;
pub const NRC_INVALID_KEY: u8 = 0x35;
pub const NRC_EXCEEDED_NUMBER_OF_ATTEMPTS: u8 = 0x36;
pub const NRC_RESPONSE_PENDING: u8 = 0x78;

/// UDS Request Message
#[derive(Debug, Clone)]
pub struct UdsRequest {
    pub service_id: u8,
    pub sub_function: u8,
    pub data: Vec<u8>,
}

/// UDS Response Message
#[derive(Debug, Clone)]
pub struct UdsResponse {
    pub response_type: UdsResponseType,
    pub service_id: u8,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl Default for UdsResponse {
    fn default() -> Self {
        Self {
            response_type: UdsResponseType::Positive,
            service_id: 0,
            data: Vec::new(),
            timestamp: 0,
        }
    }
}

/// UDS Session Status
#[derive(Debug, Clone)]
pub struct SessionStatus {
    pub session_type: UdsSessionType,
    pub security_level: u8,
    pub last_activity: std::time::Instant,
    pub tester_present_sent: bool,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self {
            session_type: UdsSessionType::Default,
            security_level: 0,
            last_activity: std::time::Instant::now(),
            tester_present_sent: false,
        }
    }
}

/// UDS Configuration
#[derive(Debug, Clone)]
pub struct UdsConfig {
    pub p2_timeout_ms: u32,              // Server response timeout
    pub p2_star_timeout_ms: u32,         // Server response pending timeout
    pub s3_client_timeout_ms: u32,       // Session timeout
    pub tester_present_interval_ms: u32, // Interval for sending tester present
}

impl Default for UdsConfig {
    fn default() -> Self {
        Self {
            p2_timeout_ms: 1000,
            p2_star_timeout_ms: 5000,
            s3_client_timeout_ms: 5000,
            tester_present_interval_ms: 2000,
        }
    }
}

impl Config for UdsConfig {
    fn validate(&self) -> Result<()> {
        if self.p2_timeout_ms == 0
            || self.p2_star_timeout_ms == 0
            || self.s3_client_timeout_ms == 0
            || self.tester_present_interval_ms == 0
        {
            return Err(AutomotiveError::InvalidParameter);
        }
        Ok(())
    }
}

/// UDS Implementation
pub struct Uds<T: TransportLayer> {
    config: UdsConfig,
    transport: T,
    pub status: SessionStatus, // Make public for testing
    is_open: bool,
}

impl<T: TransportLayer> Uds<T> {
    /// Creates a new UDS instance with the given transport layer
    pub fn with_transport(config: UdsConfig, transport: T) -> Self {
        Self {
            config,
            transport,
            status: SessionStatus::default(),
            is_open: false,
        }
    }

    /// Changes the diagnostic session
    pub fn change_session(&mut self, session_type: UdsSessionType) -> Result<()> {
        let request = UdsRequest {
            service_id: SID_DIAGNOSTIC_SESSION_CONTROL,
            sub_function: session_type as u8,
            data: vec![],
        };

        let response = self.send_request(&request)?;

        if response.response_type == UdsResponseType::Positive {
            self.status.session_type = session_type;
            self.status.last_activity = std::time::Instant::now();
            Ok(())
        } else {
            Err(AutomotiveError::UdsError("Failed to change session".into()))
        }
    }

    /// Performs ECU reset
    pub fn ecu_reset(&mut self, reset_type: UdsResetType) -> Result<()> {
        let request = UdsRequest {
            service_id: SID_ECU_RESET,
            sub_function: reset_type as u8,
            data: vec![],
        };

        let response = self.send_request(&request)?;

        if response.response_type == UdsResponseType::Positive {
            Ok(())
        } else {
            Err(AutomotiveError::UdsError("Failed to reset ECU".into()))
        }
    }

    /// Reads data by identifier
    pub fn read_data_by_id(&mut self, did: u16) -> Result<Vec<u8>> {
        let request = UdsRequest {
            service_id: SID_READ_DATA_BY_ID,
            sub_function: 0,
            data: vec![(did >> 8) as u8, did as u8],
        };

        let response = self.send_request(&request)?;

        if response.response_type == UdsResponseType::Positive {
            Ok(response.data)
        } else {
            Err(AutomotiveError::UdsError("Failed to read data".into()))
        }
    }

    /// Writes data by identifier
    pub fn write_data_by_id(&mut self, did: u16, data: &[u8]) -> Result<()> {
        let mut request_data = vec![(did >> 8) as u8, did as u8];
        request_data.extend_from_slice(data);

        let request = UdsRequest {
            service_id: SID_WRITE_DATA_BY_ID,
            sub_function: 0,
            data: request_data,
        };

        let response = self.send_request(&request)?;

        if response.response_type == UdsResponseType::Positive {
            Ok(())
        } else {
            Err(AutomotiveError::UdsError("Failed to write data".into()))
        }
    }

    /// Sends tester present message
    pub fn tester_present(&mut self) -> Result<()> {
        let request = UdsRequest {
            service_id: SID_TESTER_PRESENT,
            sub_function: 0x00,
            data: vec![],
        };

        let response = self.send_request(&request)?;

        if response.response_type == UdsResponseType::Positive {
            Ok(())
        } else {
            Err(AutomotiveError::UdsError(
                "Failed to send tester present".into(),
            ))
        }
    }

    /// Performs security access
    pub fn security_access(&mut self, level: u8, key_fn: impl Fn(&[u8]) -> Vec<u8>) -> Result<()> {
        // Request seed
        let request = UdsRequest {
            service_id: SID_SECURITY_ACCESS,
            sub_function: 2 * level - 1,
            data: vec![],
        };

        let response = self.send_request(&request)?;

        if let UdsResponseType::Positive = response.response_type {
            // Calculate key
            let key = key_fn(&response.data);

            // Send key
            let request = UdsRequest {
                service_id: SID_SECURITY_ACCESS,
                sub_function: 2 * level,
                data: key,
            };

            let response = self.send_request(&request)?;

            if response.response_type == UdsResponseType::Positive {
                self.status.security_level = level;
                self.status.last_activity = std::time::Instant::now();
                Ok(())
            } else {
                Err(AutomotiveError::UdsError("Invalid key".into()))
            }
        } else {
            Err(AutomotiveError::UdsError("Failed to get seed".into()))
        }
    }

    /// Performs routine control
    pub fn routine_control(
        &mut self,
        routine_type: u8,
        routine_id: u16,
        data: &[u8],
    ) -> Result<Vec<u8>> {
        let mut request_data = vec![(routine_id >> 8) as u8, routine_id as u8];
        request_data.extend_from_slice(data);

        let request = UdsRequest {
            service_id: SID_ROUTINE_CONTROL,
            sub_function: routine_type,
            data: request_data,
        };

        let response = self.send_request(&request)?;

        if response.response_type == UdsResponseType::Positive {
            Ok(response.data[2..].to_vec())
        } else {
            Err(AutomotiveError::UdsError("Routine control failed".into()))
        }
    }

    /// Performs input/output control
    pub fn io_control(
        &mut self,
        did: u16,
        control_param: u8,
        control_state: &[u8],
    ) -> Result<Vec<u8>> {
        let mut request_data = vec![(did >> 8) as u8, did as u8, control_param];
        request_data.extend_from_slice(control_state);

        let request = UdsRequest {
            service_id: SID_INPUT_OUTPUT_CONTROL_BY_ID,
            sub_function: 0,
            data: request_data,
        };

        let response = self.send_request(&request)?;

        if response.response_type == UdsResponseType::Positive {
            Ok(response.data[3..].to_vec())
        } else {
            Err(AutomotiveError::UdsError("IO control failed".into()))
        }
    }

    /// Reads memory by address
    pub fn read_memory(&mut self, address: u32, size: u16) -> Result<Vec<u8>> {
        let request_data = vec![
            4, // Address length
            2, // Size length
            (address >> 24) as u8,
            (address >> 16) as u8,
            (address >> 8) as u8,
            address as u8,
            (size >> 8) as u8,
            size as u8,
        ];

        let request = UdsRequest {
            service_id: SID_READ_MEMORY_BY_ADDRESS,
            sub_function: 0,
            data: request_data,
        };

        let response = self.send_request(&request)?;

        if response.response_type == UdsResponseType::Positive {
            Ok(response.data)
        } else {
            Err(AutomotiveError::UdsError("Memory read failed".into()))
        }
    }

    /// Writes memory by address
    pub fn write_memory(&mut self, address: u32, data: &[u8]) -> Result<()> {
        let mut request_data = vec![
            4, // Address length
            (data.len() as u16 >> 8) as u8,
            data.len() as u8,
            (address >> 24) as u8,
            (address >> 16) as u8,
            (address >> 8) as u8,
            address as u8,
        ];
        request_data.extend_from_slice(data);

        let request = UdsRequest {
            service_id: SID_WRITE_MEMORY_BY_ADDRESS,
            sub_function: 0,
            data: request_data,
        };

        let response = self.send_request(&request)?;

        if response.response_type == UdsResponseType::Positive {
            Ok(())
        } else {
            Err(AutomotiveError::UdsError("Memory write failed".into()))
        }
    }

    /// Handles session timing and tester present
    fn handle_session_timing(&mut self) -> Result<()> {
        let now = std::time::Instant::now();

        // Check session timeout
        if self.status.session_type != UdsSessionType::Default
            && now.duration_since(self.status.last_activity).as_millis()
                > self.config.s3_client_timeout_ms as u128
        {
            // Session timeout occurred, reset to default session
            self.status = SessionStatus::default();
            return Ok(());
        }

        // Send tester present if needed
        if self.status.session_type != UdsSessionType::Default
            && (!self.status.tester_present_sent
                || now.duration_since(self.status.last_activity).as_millis()
                    > self.config.tester_present_interval_ms as u128)
        {
            self.tester_present()?;
            self.status.tester_present_sent = true;
        }

        Ok(())
    }
}

impl<T: TransportLayer> ApplicationLayer for Uds<T> {
    type Config = UdsConfig;
    type Request = UdsRequest;
    type Response = UdsResponse;

    fn new(_config: Self::Config) -> Result<Self> {
        Err(AutomotiveError::NotInitialized)
    }

    fn open(&mut self) -> Result<()> {
        if self.is_open {
            return Ok(());
        }

        self.config.validate()?;
        self.transport.open()?;
        self.is_open = true;
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        if !self.is_open {
            return Ok(());
        }

        self.transport.close()?;
        self.is_open = false;
        self.status = SessionStatus::default();
        Ok(())
    }

    fn send_request(&mut self, request: &UdsRequest) -> Result<UdsResponse> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        let mut _pending_count = 0;

        // Handle session timing before sending request
        self.handle_session_timing()?;

        // Build request data
        let mut data = vec![request.service_id];
        if request.sub_function != 0 {
            data.push(request.sub_function);
        }
        data.extend_from_slice(&request.data);

        // Send request
        self.transport.send(&data)?;

        // Receive response with timeout handling
        let start_time = std::time::Instant::now();
        let mut response_data = None;

        while start_time.elapsed().as_millis() < self.config.p2_star_timeout_ms as u128 {
            match self.transport.receive() {
                Ok(data) => {
                    if data.len() >= 3
                        && data[0] == 0x7F
                        && data[1] == request.service_id
                        && data[2] == NRC_RESPONSE_PENDING
                    {
                        _pending_count += 1;
                        continue;
                    }
                    response_data = Some(data);
                    break;
                }
                Err(_) if start_time.elapsed().as_millis() < self.config.p2_timeout_ms as u128 => {
                    continue
                }
                Err(e) => return Err(e),
            }
        }

        let response_data =
            response_data.ok_or_else(|| AutomotiveError::UdsError("Response timeout".into()))?;

        if response_data.len() < 1 {
            return Err(AutomotiveError::UdsError("Response too short".into()));
        }

        // Update session activity
        self.status.last_activity = std::time::Instant::now();

        // Parse response
        let response_sid = response_data[0];

        if response_sid == 0x7F {
            // Negative response
            if response_data.len() < 3 {
                return Err(AutomotiveError::UdsError(
                    "Negative response too short".into(),
                ));
            }

            Ok(UdsResponse {
                response_type: UdsResponseType::Negative(response_data[2]),
                service_id: response_data[1],
                data: response_data[3..].to_vec(),
                timestamp: 0,
            })
        } else {
            // Positive response
            if response_sid != request.service_id + 0x40 {
                return Err(AutomotiveError::UdsError(
                    "Invalid response service ID".into(),
                ));
            }

            Ok(UdsResponse {
                response_type: UdsResponseType::Positive,
                service_id: request.service_id,
                data: response_data[1..].to_vec(),
                timestamp: 0,
            })
        }
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        self.transport.set_timeout(timeout_ms)
    }
}
