use super::*;
use crate::error::AutomotiveError;
use crate::isotp::{AddressMode, IsoTp, IsoTpConfig, IsoTpTiming};
use crate::physical::{mock::MockPhysical, PhysicalLayer};
use crate::types::Frame;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[test]
fn test_isotp_single_frame() -> Result<()> {
    let mut mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
        // Echo back a response with service ID + 0x40
        Ok(Frame {
            id: frame.id,
            data: vec![0x01, 0x50], // Single frame with length 1, response 0x50
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    })));
    mock.open()?;

    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        ..Default::default()
    };

    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open()?;

    // Send single frame
    isotp.send(&[0x10])?;

    // Receive response
    let response = isotp.receive()?;
    assert_eq!(response, vec![0x50]);

    Ok(())
}

#[test]
fn test_isotp_multi_frame() {
    let frame_count = Arc::new(AtomicU32::new(0));
    let frame_count_clone = frame_count.clone();

    let mock = MockPhysical::new(Some(Box::new(move |frame: &Frame| {
        frame_count_clone.fetch_add(1, Ordering::SeqCst);
        match frame.data[0] & 0xF0 {
            0x10 => {
                // First frame
                Ok(Frame {
                    id: frame.id,
                    data: vec![0x30, 0x00, 0x00], // Flow control with BS=0, STmin=0
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            0x20 => {
                // Consecutive frames
                Ok(Frame {
                    id: frame.id,
                    data: vec![0x21, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07], // Consecutive frame response
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            _ => Ok(Frame {
                id: frame.id,
                data: vec![0x01, 0x50], // Default response
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            }),
        }
    })));

    let mut mock = mock;
    mock.open().unwrap();

    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        block_size: 0, // No block size limit
        st_min: 0,     // No separation time
        timing: IsoTpTiming {
            n_as: 1000,
            n_ar: 1000,
            n_bs: 1000,
            n_cr: 1000,
        },
        ..Default::default()
    };

    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();

    // Send multi-frame message (20 bytes will require multiple frames)
    let data = vec![0x10; 20];
    isotp.send(&data).unwrap();

    // Verify frame count (should be at least 4: first frame + flow control + 2 consecutive frames)
    assert!(frame_count.load(Ordering::SeqCst) >= 4);

    isotp.close().unwrap();
}

#[test]
fn test_isotp_extended_addressing() -> Result<()> {
    let mut mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
        // Verify extended addressing
        assert_eq!(frame.data[0], 0x55); // Address extension
        Ok(Frame {
            id: frame.id,
            data: vec![0x55, 0x01, 0x50], // Address extension + single frame with length 1
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    })));
    mock.open()?;

    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        address_mode: AddressMode::Extended,
        address_extension: 0x55,
        ..Default::default()
    };

    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open()?;

    // Send single frame with extended addressing
    isotp.send(&[0x10])?;

    // Receive response
    let response = isotp.receive()?;
    assert_eq!(response, vec![0x50]);

    Ok(())
}

#[test]
fn test_isotp_mixed_addressing() -> Result<()> {
    let mut mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
        // Verify mixed addressing
        assert_eq!(frame.id & 0xFF, 0x55); // Address extension in ID
        Ok(Frame {
            id: (frame.id & 0xFFFFFF00) | 0x55, // Keep address extension
            data: vec![0x01, 0x50],             // Single frame with length 1
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    })));
    mock.open()?;

    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        address_mode: AddressMode::Mixed,
        address_extension: 0x55,
        ..Default::default()
    };

    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open()?;

    // Send single frame with mixed addressing
    isotp.send(&[0x10])?;

    Ok(())
}

#[test]
fn test_isotp_padding() -> Result<()> {
    let mut mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
        // Verify padding
        assert_eq!(frame.data.len(), 8);
        assert_eq!(&frame.data[2..], &[0xAA; 6]);
        Ok(Frame {
            id: frame.id,
            data: vec![0x01, 0x50], // Single frame with length 1, response 0x50
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    })));
    mock.open()?;

    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        use_padding: true,
        padding_value: 0xAA,
        ..Default::default()
    };

    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open()?;

    // Send single frame with padding
    isotp.send(&[0x10])?;

    // Receive response
    let response = isotp.receive()?;
    assert_eq!(response, vec![0x50]);

    Ok(())
}

#[test]
fn test_isotp_flow_control() {
    let frame_count = Arc::new(AtomicU32::new(0));
    let frame_count_clone = frame_count.clone();
    let block_count = Arc::new(AtomicU32::new(0));
    let block_count_clone = block_count.clone();

    let mock = MockPhysical::new(Some(Box::new(move |frame: &Frame| {
        frame_count_clone.fetch_add(1, Ordering::SeqCst);
        match frame.data[0] & 0xF0 {
            0x10 => {
                // First frame
                block_count_clone.store(0, Ordering::SeqCst);
                Ok(Frame {
                    id: frame.id,
                    data: vec![0x30, 0x02, 0x10], // Flow control: BS=2, ST=16ms
                    timestamp: 0,
                    is_extended: false,
                    is_fd: false,
                })
            }
            0x20 => {
                // Consecutive frames
                let current_block = block_count_clone.fetch_add(1, Ordering::SeqCst);
                if current_block >= 2 {
                    block_count_clone.store(0, Ordering::SeqCst);
                    Ok(Frame {
                        id: frame.id,
                        data: vec![0x30, 0x02, 0x10], // Flow control: BS=2, ST=16ms
                        timestamp: 0,
                        is_extended: false,
                        is_fd: false,
                    })
                } else {
                    Ok(Frame {
                        id: frame.id,
                        data: vec![0x21, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07], // Consecutive frame response
                        timestamp: 0,
                        is_extended: false,
                        is_fd: false,
                    })
                }
            }
            _ => Ok(Frame {
                id: frame.id,
                data: vec![0x01, 0x50], // Default response
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            }),
        }
    })));

    let mut mock = mock;
    mock.open().unwrap();

    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        block_size: 2, // Block size of 2 frames
        st_min: 16,    // 16ms separation time
        timing: IsoTpTiming {
            n_as: 1000,
            n_ar: 1000,
            n_bs: 1000,
            n_cr: 1000,
        },
        ..Default::default()
    };

    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();

    // Send multi-frame message (30 bytes will require multiple flow controls)
    let data = vec![0x10; 30];
    isotp.send(&data).unwrap();

    // Verify frame count (should be at least 8: first frame + flow control + 6 consecutive frames with additional flow controls)
    assert!(frame_count.load(Ordering::SeqCst) >= 8);

    isotp.close().unwrap();
}

#[test]
fn test_isotp_timeouts() -> Result<()> {
    let mut mock = MockPhysical::new(Some(Box::new(|frame: &Frame| {
        match frame.data[0] & 0xF0 {
            0x10 => {
                // First frame
                std::thread::sleep(std::time::Duration::from_millis(100));
                Err(AutomotiveError::Timeout)
            }
            _ => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                Err(AutomotiveError::Timeout)
            }
        }
    })));
    mock.open()?;

    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        timing: IsoTpTiming {
            n_as: 50, // 50ms timeout
            n_ar: 50,
            n_bs: 50,
            n_cr: 50,
        },
        ..Default::default()
    };

    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open()?;

    // Send should timeout waiting for flow control
    let data = vec![0x10; 20]; // Multi-frame message
    assert!(matches!(isotp.send(&data), Err(AutomotiveError::Timeout)));

    // Receive should timeout waiting for response
    assert!(matches!(isotp.receive(), Err(AutomotiveError::Timeout)));

    Ok(())
}

#[test]
fn test_isotp_error_handling() -> Result<()> {
    let mut mock = MockPhysical::new(Some(Box::new(|_frame: &Frame| {
        Err(AutomotiveError::NotInitialized)
    })));

    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        timing: IsoTpTiming {
            n_as: 50, // 50ms timeout
            n_ar: 50,
            n_bs: 50,
            n_cr: 50,
        },
        ..Default::default()
    };

    let mut isotp = IsoTp::with_physical(config, mock);

    // Should fail since we haven't opened the connection
    assert!(matches!(
        isotp.send(&[0x10]),
        Err(AutomotiveError::NotInitialized)
    ));
    assert!(matches!(
        isotp.receive(),
        Err(AutomotiveError::NotInitialized)
    ));

    Ok(())
}

#[test]
fn test_isotp_invalid_response() {
    let mock = MockPhysical::new(Some(Box::new(|_frame: &Frame| {
        Ok(Frame {
            id: 0,
            data: vec![0x7F, 0x00, 0x31], // Invalid response
            timestamp: 0,
            is_extended: false,
            is_fd: false,
        })
    })));

    let mut mock = mock;
    mock.open().unwrap();

    let config = IsoTpConfig {
        tx_id: 0x123,
        rx_id: 0x456,
        ..Default::default()
    };

    let mut isotp = IsoTp::with_physical(config, mock);
    isotp.open().unwrap();

    // Send should fail due to invalid response
    let data = vec![0x10; 20]; // Multi-frame message
    assert!(matches!(
        isotp.send(&data),
        Err(AutomotiveError::InvalidParameter)
    ));

    isotp.close().unwrap();
}
