pub mod j1939;

use crate::error::Result;
use crate::types::{Address, Config};

/// Network layer trait that must be implemented by J1939
pub trait NetworkLayer: Send + Sync {
    type Config: Config;
    type Message;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn send(&mut self, address: &Address, data: &[u8]) -> Result<()>;
    fn receive(&mut self) -> Result<Self::Message>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
    fn claim_address(&mut self, address: u8) -> Result<()>;
    fn get_address(&self) -> Result<u8>;
}
