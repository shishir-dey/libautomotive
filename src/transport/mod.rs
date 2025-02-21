pub mod isotp;

use crate::error::Result;
use crate::types::Config;

/// Transport layer trait that must be implemented by ISO-TP
pub trait TransportLayer: Send + Sync {
    type Config: Config;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn send(&mut self, data: &[u8]) -> Result<()>;
    fn receive(&mut self) -> Result<Vec<u8>>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
}
