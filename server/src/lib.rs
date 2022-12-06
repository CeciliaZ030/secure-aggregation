use anyhow::Result;

pub trait MessageHandler {
    /// Handshake with incoming client
    fn handle_handshake(&self) -> Result<()>;
    /// Key Exchange
    fn handle_key_exchange(&self) -> Result<()>;
    /// Input shareing
    fn handle_input_sharing(&self) -> Result<()>;
    /// Correct error based on client response
    fn handle_error_correction(&self) -> Result<()>;
}
