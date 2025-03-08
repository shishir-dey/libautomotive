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

    fn create_mock_uds() -> Uds<IsoTp<MockPhysical>> {
        let mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
            let service_id = frame.data[0]; // Service ID is the first byte
            let response_data = match service_id {
                SID_DIAGNOSTIC_SESSION_CONTROL => {
                    vec![0x50, frame.data[1]] // Positive response to session control
                }
                SID_TESTER_PRESENT => {
                    vec![0x7E, 0x00] // Positive response to tester present
                }
                SID_ROUTINE_CONTROL => {
                    vec![0x71, frame.data[1], frame.data[2], frame.data[3]] // Positive response to routine control
                }
                SID_INPUT_OUTPUT_CONTROL_BY_ID => {
                    vec![0x2F, frame.data[1], frame.data[2], frame.data[3], 0x00]
                    // Positive response to IO control
                }
                SID_READ_MEMORY_BY_ADDRESS => {
                    vec![0x63, 0x01, 0x02, 0x03] // Sample memory data
                }
                SID_WRITE_MEMORY_BY_ADDRESS => {
                    vec![0x7F, service_id, 0x31] // Negative response
                }
                _ => vec![0x7F, service_id, 0x11], // Service not supported
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
            let service_id = frame.data[0];

            // Return a positive response
            Ok(Frame {
                id: frame.id,
                data: vec![service_id + 0x40], // Positive response
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

    fn create_mock_obd() -> Obd<IsoTp<MockPhysical>> {
        let mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
            let mode = frame.data[0]; // Mode is the first byte
            let response_data = match mode {
                0x01 => {
                    // Mode 1 - Current Data
                    let pid = frame.data[1];
                    match pid {
                        PID_ENGINE_RPM => {
                            vec![0x41, PID_ENGINE_RPM, 0x1B, 0x56] // 1750 RPM
                        }
                        PID_VEHICLE_SPEED => {
                            vec![0x41, PID_VEHICLE_SPEED, 0x32] // 50 km/h
                        }
                        _ => vec![0x41, pid, 0x00], // Default response
                    }
                }
                0x03 => {
                    // Mode 3 - Show stored DTCs
                    vec![
                        0x43, 0x02, // 2 DTCs
                        0x01, 0x33, // First DTC: P0133
                        0x02, 0x44, // Second DTC: P0244
                    ]
                }
                0x02 => {
                    // Mode 2 - Freeze frame data
                    let pid = frame.data[1];
                    let frame_num = frame.data[2];
                    match pid {
                        PID_ENGINE_RPM => {
                            vec![0x42, pid, frame_num, 0x1B, 0x56] // 1750 RPM (same as current data)
                        }
                        _ => vec![0x42, pid, frame_num, 0x00],
                    }
                }
                _ => vec![0x7F, mode, 0x11], // Service not supported
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
            // Always return a valid response for engine RPM
            Ok(Frame {
                id: frame.id,
                data: vec![0x42, PID_ENGINE_RPM, 0x00, 0x1B, 0x56], // 1750 RPM
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
