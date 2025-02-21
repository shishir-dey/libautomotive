pub mod can;
pub mod canfd;
pub mod mock;

use crate::error::Result;
use crate::types::{Config, Frame};

/// Physical layer trait that must be implemented by CAN and CANFD
pub trait PhysicalLayer: Send + Sync {
    type Config: Config;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn send_frame(&mut self, frame: &Frame) -> Result<()>;
    fn receive_frame(&mut self) -> Result<Frame>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
}
