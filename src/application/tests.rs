use super::*;
use crate::transport::isotp::{IsoTp, IsoTpConfig};
use crate::physical::mock::MockPhysical;

mod uds_tests {
    use super::*;
    
    fn create_mock_uds() -> Uds<IsoTp<MockPhysical>> {
        let mock = MockPhysical::new(|frame| {
            let service_id = frame.data[0];
            Ok(Frame {
                id: frame.id,
                data: vec![service_id + 0x40], // Positive response
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })
        });
        
        let isotp_config = IsoTpConfig {
            tx_id: 0x123,
            rx_id: 0x456,
            ..Default::default()
        };
        
        let isotp = IsoTp::with_physical(isotp_config, mock);
        
        let uds_config = UdsConfig {
            p2_timeout_ms: 1000,
            p2_star_timeout_ms: 5000,
            s3_client_timeout_ms: 5000,
            tester_present_interval_ms: 2000,
        };
        
        Uds::with_transport(uds_config, isotp)
    }
    
    #[test]
    fn test_uds_session_control() {
        let mut uds = create_mock_uds();
        uds.open().unwrap();
        
        // Change to programming session
        uds.change_session(UdsSessionType::Programming).unwrap();
        assert_eq!(uds.status.session_type, UdsSessionType::Programming);
        
        // Change to extended session
        uds.change_session(UdsSessionType::Extended).unwrap();
        assert_eq!(uds.status.session_type, UdsSessionType::Extended);
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_security_access() {
        let mut uds = create_mock_uds();
        uds.open().unwrap();
        
        // Request security access level 1
        uds.security_access(1, |seed| {
            // Simple key calculation: increment each byte
            seed.iter().map(|b| b + 1).collect()
        }).unwrap();
        
        assert_eq!(uds.status.security_level, 1);
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_tester_present() {
        let mut uds = create_mock_uds();
        uds.open().unwrap();
        
        // Send tester present
        uds.tester_present().unwrap();
        assert!(uds.status.tester_present_sent);
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_read_data() {
        let mut uds = create_mock_uds();
        uds.open().unwrap();
        
        // Read data by identifier
        let data = uds.read_data_by_id(0x1234).unwrap();
        assert!(!data.is_empty());
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_write_data() {
        let mut uds = create_mock_uds();
        uds.open().unwrap();
        
        // Write data by identifier
        uds.write_data_by_id(0x1234, &[0x01, 0x02, 0x03]).unwrap();
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_routine_control() {
        let mut uds = create_mock_uds();
        uds.open().unwrap();
        
        // Start routine
        let result = uds.routine_control(0x01, 0x1234, &[0x01]).unwrap();
        assert!(!result.is_empty());
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_io_control() {
        let mut uds = create_mock_uds();
        uds.open().unwrap();
        
        // Control IO
        let result = uds.io_control(0x1234, 0x01, &[0x01]).unwrap();
        assert!(!result.is_empty());
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_memory_access() {
        let mut uds = create_mock_uds();
        uds.open().unwrap();
        
        // Read memory
        let data = uds.read_memory(0x12345678, 10).unwrap();
        assert!(!data.is_empty());
        
        // Write memory
        uds.write_memory(0x12345678, &[0x01, 0x02, 0x03]).unwrap();
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_session_timeout() {
        let mut uds = create_mock_uds();
        uds.open().unwrap();
        
        // Change to programming session
        uds.change_session(UdsSessionType::Programming).unwrap();
        
        // Wait for session timeout
        std::thread::sleep(std::time::Duration::from_millis(6000));
        
        // Send request - should reset to default session
        uds.tester_present().unwrap();
        assert_eq!(uds.status.session_type, UdsSessionType::Default);
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_error_handling() {
        let mock = MockPhysical::new(|frame| {
            let service_id = frame.data[0];
            Ok(Frame {
                id: frame.id,
                data: vec![0x7F, service_id, 0x31], // Negative response
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })
        });
        
        let isotp_config = IsoTpConfig {
            tx_id: 0x123,
            rx_id: 0x456,
            ..Default::default()
        };
        
        let isotp = IsoTp::with_physical(isotp_config, mock);
        let uds_config = UdsConfig::default();
        let mut uds = Uds::with_transport(uds_config, isotp);
        
        uds.open().unwrap();
        
        // All requests should return error
        assert!(uds.change_session(UdsSessionType::Programming).is_err());
        assert!(uds.tester_present().is_err());
        assert!(uds.read_data_by_id(0x1234).is_err());
        
        uds.close().unwrap();
    }
    
    #[test]
    fn test_uds_response_pending() {
        let response_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let response_count_clone = response_count.clone();
        
        let mock = MockPhysical::new(move |frame| {
            let count = response_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let service_id = frame.data[0];
            
            if count < 2 {
                Ok(Frame {
                    id: frame.id,
                    data: vec![0x7F, service_id, 0x78], // Response pending
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            } else {
                Ok(Frame {
                    id: frame.id,
                    data: vec![service_id + 0x40], // Positive response
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
        });
        
        let isotp_config = IsoTpConfig {
            tx_id: 0x123,
            rx_id: 0x456,
            ..Default::default()
        };
        
        let isotp = IsoTp::with_physical(isotp_config, mock);
        let uds_config = UdsConfig::default();
        let mut uds = Uds::with_transport(uds_config, isotp);
        
        uds.open().unwrap();
        
        // Request should succeed after response pending
        uds.tester_present().unwrap();
        assert!(response_count.load(std::sync::atomic::Ordering::SeqCst) > 2);
        
        uds.close().unwrap();
    }
}

mod obd_tests {
    use super::*;
    
    fn create_mock_obd() -> Obd<IsoTp<MockPhysical>> {
        let mock = MockPhysical::new(|frame| {
            let service_id = frame.data[0];
            let pid = frame.data[1];
            Ok(Frame {
                id: frame.id,
                data: vec![service_id + 0x40, pid, 0x00], // Positive response
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })
        });
        
        let isotp_config = IsoTpConfig {
            tx_id: 0x7DF,  // Standard OBD-II request ID
            rx_id: 0x7E8,  // Standard OBD-II response ID
            ..Default::default()
        };
        
        let isotp = IsoTp::with_physical(isotp_config, mock);
        
        let obd_config = ObdConfig {
            response_timeout_ms: 1000,
        };
        
        Obd::with_transport(obd_config, isotp)
    }
    
    #[test]
    fn test_obd_read_sensor() {
        let mut obd = create_mock_obd();
        obd.open().unwrap();
        
        // Read engine RPM
        let data = obd.read_sensor(PID_ENGINE_RPM).unwrap();
        assert!(!data.is_empty());
        
        // Read vehicle speed
        let data = obd.read_sensor(PID_VEHICLE_SPEED).unwrap();
        assert!(!data.is_empty());
        
        obd.close().unwrap();
    }
    
    #[test]
    fn test_obd_read_sensor_data() {
        let mut obd = create_mock_obd();
        obd.open().unwrap();
        
        // Read engine RPM with conversion
        let data = obd.read_sensor_data(PID_ENGINE_RPM).unwrap();
        match data {
            PidData::EngineRpm(_) => (),
            _ => panic!("Wrong PID data type"),
        }
        
        obd.close().unwrap();
    }
    
    #[test]
    fn test_obd_read_multiple_sensors() {
        let mut obd = create_mock_obd();
        obd.open().unwrap();
        
        // Read multiple PIDs
        let pids = vec![PID_ENGINE_RPM, PID_VEHICLE_SPEED, PID_ENGINE_LOAD];
        let data = obd.read_multiple_sensors(&pids).unwrap();
        assert_eq!(data.len(), pids.len());
        
        obd.close().unwrap();
    }
    
    #[test]
    fn test_obd_read_dtc() {
        let mock = MockPhysical::new(|frame| {
            assert_eq!(frame.data[0], SID_SHOW_STORED_DTC);
            Ok(Frame {
                id: frame.id,
                data: vec![
                    SID_SHOW_STORED_DTC + 0x40,
                    0x00,
                    0x01, 0x23, // P0123
                    0x45, 0x67, // P4567
                ],
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })
        });
        
        let isotp_config = IsoTpConfig {
            tx_id: 0x7DF,
            rx_id: 0x7E8,
            ..Default::default()
        };
        
        let isotp = IsoTp::with_physical(isotp_config, mock);
        let obd_config = ObdConfig::default();
        let mut obd = Obd::with_transport(obd_config, isotp);
        
        obd.open().unwrap();
        
        // Read DTCs
        let dtcs = obd.read_dtc().unwrap();
        assert_eq!(dtcs.len(), 2);
        assert_eq!(dtcs[0], "P0123");
        assert_eq!(dtcs[1], "P4567");
        
        obd.close().unwrap();
    }
    
    #[test]
    fn test_obd_clear_dtc() {
        let mut obd = create_mock_obd();
        obd.open().unwrap();
        
        // Clear DTCs
        obd.clear_dtc().unwrap();
        
        obd.close().unwrap();
    }
    
    #[test]
    fn test_obd_freeze_frame() {
        let mut obd = create_mock_obd();
        obd.open().unwrap();
        
        // Read freeze frame data
        let data = obd.read_freeze_frame_data(PID_ENGINE_RPM, 0x00).unwrap();
        match data {
            PidData::EngineRpm(_) => (),
            _ => panic!("Wrong PID data type"),
        }
        
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
        let obd_config = ObdConfig::default();
        let mut obd = Obd::with_transport(obd_config, isotp);
        
        obd.open().unwrap();
        
        // All requests should return error
        assert!(obd.read_sensor(PID_ENGINE_RPM).is_err());
        assert!(obd.read_dtc().is_err());
        assert!(obd.clear_dtc().is_err());
        
        obd.close().unwrap();
    }
} 