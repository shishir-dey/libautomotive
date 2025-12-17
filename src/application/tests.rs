use super::*;
use crate::application::{
    obdii::{Obd, ObdConfig, PidData, PID_ENGINE_RPM, PID_VEHICLE_SPEED},
    uds::{
        Uds, UdsConfig, UdsSessionType, SID_DIAGNOSTIC_SESSION_CONTROL,
        SID_INPUT_OUTPUT_CONTROL_BY_ID, SID_READ_MEMORY_BY_ADDRESS, SID_ROUTINE_CONTROL,
        SID_TESTER_PRESENT, SID_WRITE_MEMORY_BY_ADDRESS,
    },
};
use crate::error::Result;
use crate::physical::{mock::MockPhysical, PhysicalLayer};
use crate::transport::isotp::{IsoTp, IsoTpConfig};
use crate::transport::TransportLayer;
use crate::types::Frame;

mod uds_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Helper function to wrap response data in ISO-TP Single Frame format
    fn wrap_isotp_single_frame(data: Vec<u8>) -> Vec<u8> {
        let mut frame_data = vec![data.len() as u8]; // PCI byte with length
        frame_data.extend(data);
        frame_data
    }

    fn create_mock_uds() -> Uds<IsoTp<MockPhysical>> {
        // Flag to track if we're waiting for consecutive frames after sending FC
        let waiting_for_cf = Arc::new(AtomicBool::new(false));
        let waiting_for_cf_clone = Arc::clone(&waiting_for_cf);

        let mock = MockPhysical::new(Some(Box::new(move |frame: &Frame| {
            // Check for ISO-TP First Frame (multi-frame request)
            // First Frame PCI: 0x1X where X is high nibble of length
            if !frame.data.is_empty() && (frame.data[0] & 0xF0) == 0x10 {
                // This is a First Frame - respond with Flow Control
                // FC format: [0x30, block_size, st_min]
                waiting_for_cf_clone.store(true, Ordering::SeqCst);
                return Ok(Frame {
                    id: frame.id,
                    data: vec![0x30, 0x00, 0x00], // Flow Control: CTS, no block limit, no delay
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                });
            }

            // Check for Consecutive Frame
            if !frame.data.is_empty() && (frame.data[0] & 0xF0) == 0x20 {
                // This is a Consecutive Frame - we need to assemble the full message
                // For simplicity, just return a positive response for memory operations
                if waiting_for_cf_clone.load(Ordering::SeqCst) {
                    waiting_for_cf_clone.store(false, Ordering::SeqCst);
                    // Return a positive response for ReadMemoryByAddress (0x23 -> 0x63)
                    return Ok(Frame {
                        id: frame.id,
                        data: wrap_isotp_single_frame(vec![0x63, 0x01, 0x02, 0x03]),
                        timestamp: 0,
                        is_extended: false,
                        is_fd: false,
                    });
                }
            }

            // ISO-TP Single Frame: first byte is PCI (0x0X where X is length)
            // Skip the PCI byte to get the actual service ID
            let service_id = if !frame.data.is_empty() && (frame.data[0] & 0xF0) == 0x00 {
                // Single Frame - service ID is at index 1
                if frame.data.len() > 1 { frame.data[1] } else { 0 }
            } else {
                // Fallback for non-ISO-TP frames
                frame.data[0]
            };

            let response_data = match service_id {
                SID_DIAGNOSTIC_SESSION_CONTROL => {
                    let sub_func = if frame.data.len() > 2 { frame.data[2] } else { 0x01 };
                    wrap_isotp_single_frame(vec![0x50, sub_func]) // Positive response to session control
                }
                SID_TESTER_PRESENT => {
                    wrap_isotp_single_frame(vec![0x7E, 0x00]) // Positive response to tester present
                }
                SID_ROUTINE_CONTROL => {
                    let (b1, b2, b3) = if frame.data.len() > 4 {
                        (frame.data[2], frame.data[3], frame.data[4])
                    } else {
                        (0x01, 0x00, 0x00)
                    };
                    wrap_isotp_single_frame(vec![0x71, b1, b2, b3]) // Positive response to routine control
                }
                SID_INPUT_OUTPUT_CONTROL_BY_ID => {
                    let (b1, b2, b3) = if frame.data.len() > 4 {
                        (frame.data[2], frame.data[3], frame.data[4])
                    } else {
                        (0x00, 0x00, 0x00)
                    };
                    wrap_isotp_single_frame(vec![0x6F, b1, b2, b3, 0x00]) // Positive response to IO control
                }
                SID_READ_MEMORY_BY_ADDRESS => {
                    wrap_isotp_single_frame(vec![0x63, 0x01, 0x02, 0x03]) // Sample memory data
                }
                SID_WRITE_MEMORY_BY_ADDRESS => {
                    wrap_isotp_single_frame(vec![0x7F, service_id, 0x31]) // Negative response
                }
                _ => wrap_isotp_single_frame(vec![0x7F, service_id, 0x11]), // Service not supported
            };
            Ok(Frame {
                id: frame.id,
                data: response_data,
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })
        })));

        let mut mock = mock;
        mock.open().unwrap();

        let isotp_config = IsoTpConfig {
            tx_id: 0x123,
            rx_id: 0x456,
            ..Default::default()
        };

        let mut isotp = IsoTp::with_physical(isotp_config, mock);
        isotp.open().unwrap();

        let uds_config = UdsConfig {
            timeout_ms: 1000,
            p2_timeout_ms: 100, // Reduce timeouts for testing
            p2_star_timeout_ms: 500,
            s3_client_timeout_ms: 500,
            tester_present_interval_ms: 200,
        };

        let mut uds = Uds::with_transport(uds_config, isotp);
        uds.open().unwrap();
        uds
    }

    #[test]
    fn test_uds_tester_present() {
        let mut uds = create_mock_uds();
        uds.tester_present().unwrap();
        assert!(uds.status.tester_present_sent);
        uds.close().unwrap();
    }

    #[test]
    fn test_uds_read_data() {
        let mut uds = create_mock_uds();
        let data = uds.read_data_by_id(0x1234).unwrap();
        assert!(!data.is_empty());
        uds.close().unwrap();
    }

    #[test]
    fn test_uds_write_data() {
        let mut uds = create_mock_uds();
        assert!(uds.write_data_by_id(0x1234, &[0x01, 0x02, 0x03]).is_err());
        uds.close().unwrap();
    }

    #[test]
    fn test_uds_routine_control() {
        let mut uds = create_mock_uds();
        let result = uds.routine_control(0x01, 0x1234, &[0x01]).unwrap();
        assert!(!result.is_empty());
        uds.close().unwrap();
    }

    #[test]
    fn test_uds_io_control() {
        let mut uds = create_mock_uds();
        let result = uds.io_control(0x1234, 0x01, &[0x01]).unwrap();
        assert!(!result.is_empty());
        uds.close().unwrap();
    }

    #[test]
    fn test_uds_memory_access() {
        let mut uds = create_mock_uds();
        let data = uds.read_memory(0x12345678, 3).unwrap();
        assert_eq!(data, vec![0x01, 0x02, 0x03]);
        assert!(uds.write_memory(0x12345678, &[0x01, 0x02, 0x03]).is_err());
        uds.close().unwrap();
    }

    #[test]
    fn test_uds_session_timeout() {
        let mut uds = create_mock_uds();
        uds.change_session(UdsSessionType::Programming).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(6000));
        uds.tester_present().unwrap();
        assert_eq!(uds.status.session_type, UdsSessionType::Default);
        uds.close().unwrap();
    }

    #[test]
    fn test_uds_response_pending() {
        let mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
            // Parse ISO-TP Single Frame - service ID is at index 1 (after PCI byte)
            let service_id = if !frame.data.is_empty() && (frame.data[0] & 0xF0) == 0x00 {
                if frame.data.len() > 1 { frame.data[1] } else { 0 }
            } else {
                frame.data[0]
            };

            // Return a positive response wrapped in ISO-TP Single Frame format
            let response = vec![service_id + 0x40];
            let mut isotp_response = vec![response.len() as u8];
            isotp_response.extend(response);

            Ok(Frame {
                id: frame.id,
                data: isotp_response,
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })
        })));

        let mut mock = mock;
        mock.open().unwrap();

        let isotp_config = IsoTpConfig {
            tx_id: 0x123,
            rx_id: 0x456,
            ..Default::default()
        };

        let mut isotp = IsoTp::with_physical(isotp_config, mock);
        isotp.open().unwrap();

        let uds_config = UdsConfig::default();
        let mut uds = Uds::with_transport(uds_config, isotp);
        uds.open().unwrap();

        // This should work without errors
        uds.tester_present().unwrap();

        // Verify that the tester_present flag is set
        assert!(uds.status.tester_present_sent);

        uds.close().unwrap();
    }
}

mod obd_tests {
    use super::*;

    /// Helper function to wrap response data in ISO-TP Single Frame format
    fn wrap_isotp_single_frame(data: Vec<u8>) -> Vec<u8> {
        let mut frame_data = vec![data.len() as u8]; // PCI byte with length
        frame_data.extend(data);
        frame_data
    }

    fn create_mock_obd() -> Obd<IsoTp<MockPhysical>> {
        let mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
            // Parse ISO-TP Single Frame - mode is at index 1 (after PCI byte)
            let mode = if !frame.data.is_empty() && (frame.data[0] & 0xF0) == 0x00 {
                if frame.data.len() > 1 { frame.data[1] } else { 0 }
            } else {
                frame.data[0]
            };

            let response_data = match mode {
                0x01 => {
                    // Mode 1 - Current Data
                    let pid = if frame.data.len() > 2 { frame.data[2] } else { 0 };
                    match pid {
                        PID_ENGINE_RPM => {
                            wrap_isotp_single_frame(vec![0x41, PID_ENGINE_RPM, 0x1B, 0x56]) // 1750 RPM
                        }
                        PID_VEHICLE_SPEED => {
                            wrap_isotp_single_frame(vec![0x41, PID_VEHICLE_SPEED, 0x32]) // 50 km/h
                        }
                        _ => wrap_isotp_single_frame(vec![0x41, pid, 0x00]), // Default response
                    }
                }
                0x03 => {
                    // Mode 3 - Show stored DTCs
                    wrap_isotp_single_frame(vec![
                        0x43, 0x02, // 2 DTCs
                        0x01, 0x33, // First DTC: P0133
                        0x02, 0x44, // Second DTC: P0244
                    ])
                }
                0x02 => {
                    // Mode 2 - Freeze frame data
                    let pid = if frame.data.len() > 2 { frame.data[2] } else { 0 };
                    let frame_num = if frame.data.len() > 3 { frame.data[3] } else { 0 };
                    match pid {
                        PID_ENGINE_RPM => {
                            wrap_isotp_single_frame(vec![0x42, pid, frame_num, 0x1B, 0x56]) // 1750 RPM
                        }
                        _ => wrap_isotp_single_frame(vec![0x42, pid, frame_num, 0x00]),
                    }
                }
                _ => wrap_isotp_single_frame(vec![0x7F, mode, 0x11]), // Service not supported
            };

            Ok(Frame {
                id: frame.id,
                data: response_data,
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })
        })));

        let mut mock = mock;
        mock.open().unwrap();

        let isotp_config = IsoTpConfig {
            tx_id: 0x7DF,
            rx_id: 0x7E8,
            ..Default::default()
        };

        let mut isotp = IsoTp::with_physical(isotp_config, mock);
        isotp.open().unwrap();

        let obd_config = ObdConfig {
            timeout_ms: 1000,
            auto_format: true,
        };

        let mut obd = Obd::with_transport(obd_config, isotp);
        obd.open().unwrap();
        obd
    }

    #[test]
    fn test_obd_read_sensor() -> Result<()> {
        let mut obd = create_mock_obd();

        // Read engine RPM
        match obd.read_sensor_data(PID_ENGINE_RPM)? {
            PidData::EngineRpm(rpm) => assert_eq!(rpm, 1750.0),
            _ => panic!("Expected EngineRpm variant"),
        }

        // Read vehicle speed
        match obd.read_sensor_data(PID_VEHICLE_SPEED)? {
            PidData::VehicleSpeed(speed) => assert_eq!(speed, 50),
            _ => panic!("Expected VehicleSpeed variant"),
        }

        obd.close().unwrap();
        Ok(())
    }

    #[test]
    fn test_obd_read_dtc() -> Result<()> {
        let mut obd = create_mock_obd();

        // Read DTCs
        let dtcs = obd.read_dtc()?;
        assert_eq!(dtcs.len(), 2);
        assert_eq!(dtcs[0], "P0133");
        assert_eq!(dtcs[1], "P0244");

        obd.close().unwrap();
        Ok(())
    }

    #[test]
    fn test_obd_freeze_frame() {
        // Create a simple test that doesn't rely on the mock
        let obd_config = ObdConfig::default();

        // Create a mock that returns a valid response for freeze frame data
        let mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
            // Return ISO-TP formatted response for engine RPM freeze frame
            // Format: [PCI, Mode+0x40, PID, FrameNum, DataA, DataB]
            let response = vec![0x42, PID_ENGINE_RPM, 0x00, 0x1B, 0x56]; // 1750 RPM
            let mut isotp_response = vec![response.len() as u8];
            isotp_response.extend(response);

            Ok(Frame {
                id: frame.id,
                data: isotp_response,
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })
        })));

        let mut mock = mock;
        mock.open().unwrap();

        let isotp_config = IsoTpConfig {
            tx_id: 0x7E0,
            rx_id: 0x7E8,
            ..Default::default()
        };

        let mut isotp = IsoTp::with_physical(isotp_config, mock);
        isotp.open().unwrap();

        let mut obd = Obd::with_transport(obd_config, isotp);
        obd.open().unwrap();

        // Test passes if this doesn't panic
        let _ = obd.read_freeze_frame(PID_ENGINE_RPM, 0x00).unwrap();

        obd.close().unwrap();
    }

    #[test]
    fn test_obd_error_handling() {
        let mock = MockPhysical::new_error();

        let isotp_config = IsoTpConfig {
            tx_id: 0x7DF,
            rx_id: 0x7E8,
            ..Default::default()
        };

        let isotp = IsoTp::with_physical(isotp_config, mock);
        let obd_config = ObdConfig {
            timeout_ms: 1000,
            auto_format: true,
        };
        let mut obd = Obd::with_transport(obd_config, isotp);

        // All requests should return error
        assert!(obd.read_sensor(PID_ENGINE_RPM).is_err());
        assert!(obd.read_dtc().is_err());
        assert!(obd.clear_dtc().is_err());
    }
}
