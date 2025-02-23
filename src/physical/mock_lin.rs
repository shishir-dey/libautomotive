use crate::error::AutomotiveError;
use crate::types::Frame;
use std::sync::{Arc, Mutex};

pub struct MockLinPhysical {
    response_fn: Option<Box<dyn Fn(&Frame) -> Result<Frame, AutomotiveError> + Send>>,
    last_sent: Arc<Mutex<Option<Frame>>>,
}

impl MockLinPhysical {
    pub fn new(response_fn: Option<Box<dyn Fn(&Frame) -> Result<Frame, AutomotiveError> + Send>>) -> Self {
        MockLinPhysical {
            response_fn,
            last_sent: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new_echo() -> Self {
        MockLinPhysical::new(Some(Box::new(|frame: &Frame| {
            Ok(Frame {
                id: frame.id,
                data: frame.data.clone(),
                timestamp: 0,
                is_extended: false,
                is_fd: false,
            })
        })))
    }

    pub fn new_error() -> Self {
        MockLinPhysical::new(Some(Box::new(|_frame: &Frame| {
            Err(AutomotiveError::NotInitialized)
        })))
    }

    pub fn get_last_sent(&self) -> Option<Frame> {
        self.last_sent.lock().unwrap().clone()
    }
}

impl crate::physical::PhysicalLayer for MockLinPhysical {
    fn open(&mut self) -> Result<(), AutomotiveError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), AutomotiveError> {
        Ok(())
    }

    fn write_frame(&mut self, frame: &Frame) -> Result<(), AutomotiveError> {
        *self.last_sent.lock().unwrap() = Some(frame.clone());
        Ok(())
    }

    fn read_frame(&mut self) -> Result<Frame, AutomotiveError> {
        if let Some(last_frame) = self.last_sent.lock().unwrap().as_ref() {
            if let Some(response_fn) = &self.response_fn {
                response_fn(last_frame)
            } else {
                Err(AutomotiveError::NotInitialized)
            }
        } else {
            Err(AutomotiveError::NotInitialized)
        }
    }
} 