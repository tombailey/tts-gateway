use thiserror::Error;

#[derive(Clone, Error, Debug)]
pub enum Error {
    #[error("Invalid max_clients")]
    InvalidMaxClients,
    #[error("Unexpected API response {0}")]
    UnexpectedApiResponse(String),
    #[error("Invalid api key")]
    InvalidApiKey,
    #[error("Invalid model")]
    InvalidModel,
    #[error("Rate limit")]
    RateLimit,
    #[error("Unknown: {0}")]
    Unknown(String),
}
