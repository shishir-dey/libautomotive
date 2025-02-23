use super::PhysicalLayer;
use crate::error::{AutomotiveError, Result};
use crate::types::{Config, Frame};
use std::sync::Arc;
use std::sync::Mutex;

/// Mock frame handler function type
pub type MockFrameHandler = Box<dyn Fn(&Frame) -> Result<Frame> + Send + Sync>;

#[derive(Debug, Default)]
pub struct MockConfig {
    pub timeout_ms: u32,
}

impl Config for MockConfig {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// Mock physical layer for testing
pub struct MockPhysical {
    config: MockConfig,
    frame_handler: Option<MockFrameHandler>,
    is_open: bool,
    last_frame: Arc<Mutex<Option<Frame>>>,
}

impl MockPhysical {
    /// Creates a new mock physical layer with a custom frame handler
    pub fn new(frame_handler: Option<MockFrameHandler>) -> Self {
        Self {
            config: MockConfig::default(),
            frame_handler,
            is_open: false,
            last_frame: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a new mock physical layer with an echo handler
    pub fn new_echo() -> Self {
        Self::new(Some(Box::new(|frame: &Frame| Ok(frame.clone()))))
    }

    /// Creates a new mock physical layer that simulates errors
    pub fn new_error() -> Self {
        Self::new(Some(Box::new(|_: &Frame| {
            Err(AutomotiveError::NotInitialized)
        })))
    }

    /// Sets a new frame handler
    pub fn set_frame_handler(&mut self, handler: Option<MockFrameHandler>) {
        self.frame_handler = handler;
    }
}

impl PhysicalLayer for MockPhysical {
    type Config = MockConfig;

    fn new(config: Self::Config) -> Result<Self> {
        Ok(Self {
            config,
            frame_handler: None,
            is_open: false,
            last_frame: Arc::new(Mutex::new(None)),
        })
    }

    fn open(&mut self) -> Result<()> {
        self.is_open = true;
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        self.is_open = false;
        Ok(())
    }

    fn send_frame(&mut self, frame: &Frame) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        // Store the frame for receive_frame to use
        if let Ok(mut last_frame) = self.last_frame.lock() {
            *last_frame = Some(frame.clone());
        }
        Ok(())
    }

    fn receive_frame(&mut self) -> Result<Frame> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }

        if let Some(handler) = &self.frame_handler {
            let last_frame = self.last_frame.lock().unwrap();
            if let Some(frame) = last_frame.as_ref() {
                // Create a response frame based on the last sent frame
                let response = handler(frame)?;
                return Ok(response);
            }

            // If no frame was sent, create a default single frame response
            let default_frame = Frame {
                id: 0x7E8,
                data: vec![0x00, 0x00], // Single frame with 0 length
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            };
            return handler(&default_frame);
        }
        Err(AutomotiveError::NotInitialized)
    }

    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()> {
        if !self.is_open {
            return Err(AutomotiveError::NotInitialized);
        }
        self.config.timeout_ms = timeout_ms;
        Ok(())
    }
}
