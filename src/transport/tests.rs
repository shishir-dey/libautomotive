use super::*;
use crate::physical::mock::MockPhysical;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[test]
fn test_isotp_single_frame() {
    let mock = MockPhysical::new(|frame| {
        // Echo back a response with service ID + 0x40
        Ok(Frame {
            id: frame.id,
            data: vec![frame.data[0] | 0x40],
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    });
    
    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        ..Default::default()
    };
    
    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();
    
    // Send single frame
    isotp.send(&[0x10]).unwrap();
    
    // Receive response
    let response = isotp.receive().unwrap();
    assert_eq!(response, vec![0x50]);
    
    isotp.close().unwrap();
}

#[test]
fn test_isotp_multi_frame() {
    let frame_count = Arc::new(AtomicU32::new(0));
    let frame_count_clone = frame_count.clone();
    
    let mock = MockPhysical::new(move |frame| {
        let count = frame_count_clone.fetch_add(1, Ordering::SeqCst);
        match count {
            0 => {
                // First frame -> respond with flow control
                assert_eq!(frame.data[0] & 0xF0, 0x10); // First frame PCI
                Ok(Frame {
                    id: frame.id,
                    data: vec![0x30, 0x00, 0x00], // Flow control
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            _ => {
                // Consecutive frames -> respond with positive response
                assert_eq!(frame.data[0] & 0xF0, 0x20); // Consecutive frame PCI
                Ok(Frame {
                    id: frame.id,
                    data: vec![0x50], // Positive response
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
        }
    });
    
    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        ..Default::default()
    };
    
    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();
    
    // Send multi-frame message
    let data = vec![0x10; 10]; // 10 bytes will require multi-frame
    isotp.send(&data).unwrap();
    
    // Verify frame count
    assert!(frame_count.load(Ordering::SeqCst) > 1);
    
    isotp.close().unwrap();
}

#[test]
fn test_isotp_extended_addressing() {
    let mock = MockPhysical::new(|frame| {
        // Verify extended addressing
        assert_eq!(frame.data[0], 0x55); // Address extension
        Ok(Frame {
            id: frame.id,
            data: vec![0x55, 0x50], // Address extension + response
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    });
    
    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        address_mode: AddressMode::Extended,
        address_extension: 0x55,
        ..Default::default()
    };
    
    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();
    
    // Send single frame with extended addressing
    isotp.send(&[0x10]).unwrap();
    
    // Receive response
    let response = isotp.receive().unwrap();
    assert_eq!(response, vec![0x50]);
    
    isotp.close().unwrap();
}

#[test]
fn test_isotp_mixed_addressing() {
    let mock = MockPhysical::new(|frame| {
        // Verify mixed addressing
        assert_eq!(frame.id & 0xFF, 0x55); // Address extension in ID
        Ok(Frame {
            id: (frame.id & 0xFFFFFF00) | 0x55, // Keep address extension
            data: vec![0x50], // Response
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    });
    
    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        address_mode: AddressMode::Mixed,
        address_extension: 0x55,
        ..Default::default()
    };
    
    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();
    
    // Send single frame with mixed addressing
    isotp.send(&[0x10]).unwrap();
    
    // Receive response
    let response = isotp.receive().unwrap();
    assert_eq!(response, vec![0x50]);
    
    isotp.close().unwrap();
}

#[test]
fn test_isotp_padding() {
    let mock = MockPhysical::new(|frame| {
        // Verify padding
        assert_eq!(frame.data.len(), 8);
        assert_eq!(&frame.data[2..], &[0xAA; 6]);
        Ok(Frame {
            id: frame.id,
            data: vec![0x50],
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    });
    
    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        use_padding: true,
        padding_value: 0xAA,
        ..Default::default()
    };
    
    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();
    
    // Send single frame with padding
    isotp.send(&[0x10]).unwrap();
    
    isotp.close().unwrap();
}

#[test]
fn test_isotp_flow_control() {
    let frame_count = Arc::new(AtomicU32::new(0));
    let frame_count_clone = frame_count.clone();
    
    let mock = MockPhysical::new(move |frame| {
        let count = frame_count_clone.fetch_add(1, Ordering::SeqCst);
        match count {
            0 => {
                // First frame -> respond with flow control
                assert_eq!(frame.data[0] & 0xF0, 0x10); // First frame PCI
                Ok(Frame {
                    id: frame.id,
                    data: vec![0x30, 0x02, 0x10], // Flow control: BS=2, ST=16ms
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            2 => {
                // After 2 frames -> send another flow control
                Ok(Frame {
                    id: frame.id,
                    data: vec![0x30, 0x02, 0x10], // Flow control: BS=2, ST=16ms
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            _ => {
                // Consecutive frames -> respond with positive response
                assert_eq!(frame.data[0] & 0xF0, 0x20); // Consecutive frame PCI
                Ok(Frame {
                    id: frame.id,
                    data: vec![0x50], // Positive response
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
        }
    });
    
    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        ..Default::default()
    };
    
    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();
    
    // Send multi-frame message
    let data = vec![0x10; 20]; // 20 bytes will require multiple flow controls
    isotp.send(&data).unwrap();
    
    // Verify frame count
    assert!(frame_count.load(Ordering::SeqCst) > 3);
    
    isotp.close().unwrap();
}

#[test]
fn test_isotp_timeouts() {
    let mock = MockPhysical::new(|_| {
        std::thread::sleep(std::time::Duration::from_millis(100));
        Err(AutomotiveError::PhysicalError("Timeout".into()))
    });
    
    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        timing: IsoTpTiming {
            n_as: 50,  // 50ms timeout
            n_ar: 50,
            n_bs: 50,
            n_cr: 50,
        },
        ..Default::default()
    };
    
    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();
    
    // Send should timeout
    assert!(isotp.send(&[0x10]).is_err());
    
    // Receive should timeout
    assert!(isotp.receive().is_err());
    
    isotp.close().unwrap();
}

#[test]
fn test_isotp_error_handling() {
    let mock = MockPhysical::new_error();
    
    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        ..Default::default()
    };
    
    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();
    
    // Send should return error
    assert!(isotp.send(&[0x10]).is_err());
    
    // Receive should return error
    assert!(isotp.receive().is_err());
    
    isotp.close().unwrap();
}

#[test]
fn test_isotp_invalid_response() {
    let mock = MockPhysical::new(|_| {
        Ok(Frame {
            id: 0,
            data: vec![0xFF], // Invalid PCI
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    });
    
    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        ..Default::default()
    };
    
    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();
    
    // Receive should return error due to invalid PCI
    assert!(isotp.receive().is_err());
    
    isotp.close().unwrap();
} 