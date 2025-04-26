use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Missing object")]
    MissingObject,
    #[error("Missing config: {0}")]
    MissingConfig(String),
    #[error("Unexpected API response: {0}")]
    UnexpectedApiResponse(String),
    #[error("Unknown: {0}")]
    Unknown(String),
}
