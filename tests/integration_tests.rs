use libautomotive::application::{
    obdii::{Obd, ObdConfig, PidData, PID_ENGINE_RPM},
    uds::{
        Uds, UdsConfig, UdsSessionType, SID_DIAGNOSTIC_SESSION_CONTROL, SID_READ_DATA_BY_ID,
        SID_SECURITY_ACCESS,
    },
    ApplicationLayer,
};
use libautomotive::error::AutomotiveError;
use libautomotive::physical::mock::MockPhysical;
use libautomotive::transport::isotp::{IsoTp, IsoTpConfig};
use libautomotive::transport::TransportLayer;
use libautomotive::types::Frame;

#[test]
fn test_full_stack_uds() {
    // Create mock physical layer that simulates ECU responses
    let mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
        let data = &frame.data;
        match data[0] {
            SID_DIAGNOSTIC_SESSION_CONTROL => {
                Ok(Frame {
                    id: 0x7E8,
                    data: vec![0x50, 0x01], // Positive response
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            SID_SECURITY_ACCESS => {
                if data[1] == 0x01 {
                    Ok(Frame {
                        id: 0x7E8,
                        data: vec![0x67, 0x01, 0x01, 0x02, 0x03], // Seed
                        timestamp: 0,
                        is_extended: false,
                        is_fd: false,
                    })
                } else {
                    Ok(Frame {
                        id: 0x7E8,
                        data: vec![0x67, 0x02], // Key accepted
                        timestamp: 0,
                        is_extended: false,
                        is_fd: false,
                    })
                }
            }
            SID_READ_DATA_BY_ID => {
                Ok(Frame {
                    id: 0x7E8,
                    data: vec![0x62, 0xF1, 0x90, 0x12, 0x34], // VIN data
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            _ => Err(AutomotiveError::NotInitialized),
        }
    })));

    // Create transport layer
    let config = IsoTpConfig::default();
    let mut transport = IsoTp::with_physical(config, mock);
    transport.open().unwrap();

    // Create UDS layer
    let mut uds = Uds::with_transport(UdsConfig::default(), transport);
    uds.open().unwrap();

    // Test diagnostic session control
    uds.change_session(UdsSessionType::Programming).unwrap();
    assert_eq!(uds.status.session_type, UdsSessionType::Programming);

    // Test security access
    uds.security_access(1, |seed| {
        // Simple key calculation for testing
        seed.iter().map(|x| x + 1).collect()
    })
    .unwrap();
    assert_eq!(uds.status.security_level, 1);

    // Test read data by identifier
    let vin_data = uds.read_data_by_id(0xF190).unwrap();
    assert_eq!(vin_data, vec![0x12, 0x34]);

    uds.close().unwrap();
}

#[test]
fn test_full_stack_obd() {
    // Create mock physical layer that simulates OBD-II responses
    let mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
        let data = &frame.data;
        match data[0] {
            0x01 => {
                // Mode 1
                match data[1] {
                    0x0C => {
                        // Engine RPM
                        Ok(Frame {
                            id: 0x7E8,
                            data: vec![0x41, 0x0C, 0x1B, 0x56], // 1750 RPM
                            timestamp: 0,
                            is_extended: false,
                            is_fd: false,
                        })
                    }
                    _ => Err(AutomotiveError::NotInitialized),
                }
            }
            0x03 => {
                // Mode 3 (Get DTCs)
                Ok(Frame {
                    id: 0x7E8,
                    data: vec![0x43, 0x02, 0x01, 0x43, 0x02, 0x44], // 2 DTCs
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            _ => Err(AutomotiveError::NotInitialized),
        }
    })));

    // Create transport layer
    let config = IsoTpConfig::default();
    let mut transport = IsoTp::with_physical(config, mock);
    transport.open().unwrap();

    // Create OBD layer
    let mut obd = Obd::with_transport(ObdConfig::default(), transport);
    obd.open().unwrap();

    // Test reading engine RPM
    let rpm = obd.read_sensor_data(PID_ENGINE_RPM).unwrap();
    match rpm {
        PidData::EngineRpm(value) => assert_eq!(value, 1750.0),
        _ => panic!("Unexpected PID data type"),
    }

    // Test reading DTCs
    let dtcs = obd.read_dtc().unwrap();
    assert_eq!(dtcs.len(), 2);
    assert_eq!(dtcs[0], "P0143");
    assert_eq!(dtcs[1], "P0244");

    obd.close().unwrap();
}

#[test]
fn test_full_stack_error_handling() {
    // Create mock physical layer that simulates errors
    let mock = MockPhysical::new_error();

    // Create transport layer
    let config = IsoTpConfig::default();
    let mut transport = IsoTp::with_physical(config, mock);
    transport.open().unwrap();

    // Create UDS layer
    let mut uds = Uds::with_transport(UdsConfig::default(), transport);
    uds.open().unwrap();

    // Test error handling
    let result = uds.change_session(UdsSessionType::Programming);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AutomotiveError::NotInitialized
    ));

    uds.close().unwrap();
}

#[test]
fn test_full_stack_multi_layer() {
    // Create mock physical layer that simulates both UDS and OBD-II responses
    let mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
        let data = &frame.data;
        match data[0] {
            SID_DIAGNOSTIC_SESSION_CONTROL => {
                Ok(Frame {
                    id: 0x7E8,
                    data: vec![0x50, 0x01], // Positive response
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            0x01 => {
                // OBD-II Mode 1
                match data[1] {
                    0x0C => {
                        // Engine RPM
                        Ok(Frame {
                            id: 0x7E8,
                            data: vec![0x41, 0x0C, 0x1B, 0x56], // 1750 RPM
                            timestamp: 0,
                            is_extended: false,
                            is_fd: false,
                        })
                    }
                    _ => Err(AutomotiveError::NotInitialized),
                }
            }
            _ => Err(AutomotiveError::NotInitialized),
        }
    })));

    // Test UDS
    let config = IsoTpConfig::default();
    let mut transport = IsoTp::with_physical(config, mock);
    transport.open().unwrap();
    let mut uds = Uds::with_transport(UdsConfig::default(), transport);
    uds.open().unwrap();
    uds.change_session(UdsSessionType::Programming).unwrap();
    assert_eq!(uds.status.session_type, UdsSessionType::Programming);
    uds.close().unwrap();

    // Test OBD-II
    let config = IsoTpConfig::default();
    let mut transport = IsoTp::with_physical(config, MockPhysical::new_echo());
    transport.open().unwrap();
    let mut obd = Obd::with_transport(ObdConfig::default(), transport);
    obd.open().unwrap();
    let rpm = obd.read_sensor_data(PID_ENGINE_RPM).unwrap();
    match rpm {
        PidData::EngineRpm(value) => assert_eq!(value, 1750.0),
        _ => panic!("Unexpected PID data type"),
    }
    obd.close().unwrap();
}
