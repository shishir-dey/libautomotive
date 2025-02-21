pub mod obdii;
pub mod uds;

use crate::error::Result;
use crate::types::Config;

/// Application layer trait that must be implemented by UDS and OBD-II
pub trait ApplicationLayer: Send + Sync {
    type Config: Config;
    type Request;
    type Response;

    fn new(config: Self::Config) -> Result<Self>
    where
        Self: Sized;
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn send_request(&mut self, request: &Self::Request) -> Result<Self::Response>;
    fn set_timeout(&mut self, timeout_ms: u32) -> Result<()>;
}
