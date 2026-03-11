use crate::audio_config::AudioConfig;
use crate::error::Error;
use crate::voice::Voice;
use base64::Engine as _;
use base64::prelude::*;
use bytes::Bytes;
use env::{env_var, require_env_var_or};
use futures::Stream;
use rand::prelude::IteratorRandom;
use reqwest::{StatusCode, header};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::pin::Pin;
use tokio::sync::{Semaphore, SemaphorePermit, TryAcquireError};

struct ClientSemaphorePair {
    client: ClientWithMiddleware,
    semaphore: Semaphore,
}

pub struct GcpClient {
    api_key: String,
    clients: Vec<ClientSemaphorePair>,
}

pub const GOOGLE_API_KEY: &str = "GOOGLE_API_KEY";
pub const GOOGLE_REFERRER: &str = "GOOGLE_REFERRER";

pub const REFERRER: &str = "Referer";

impl From<tokio::sync::AcquireError> for Error {
    fn from(value: tokio::sync::AcquireError) -> Self {
        Error::Unknown(value.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::Unknown(value.to_string())
    }
}

impl From<header::InvalidHeaderValue> for Error {
    fn from(value: header::InvalidHeaderValue) -> Self {
        Error::Unknown(value.to_string())
    }
}

impl From<reqwest_middleware::Error> for Error {
    fn from(value: reqwest_middleware::Error) -> Self {
        Error::Unknown(value.to_string())
    }
}

impl From<base64::DecodeError> for Error {
    fn from(value: base64::DecodeError) -> Self {
        Error::Unknown(value.to_string())
    }
}

fn create_client(
    default_headers: header::HeaderMap,
    max_retries: u32,
) -> Result<ClientWithMiddleware, Error> {
    let client = reqwest::ClientBuilder::new()
        .default_headers(default_headers)
        .build()?;
    Ok(match max_retries {
        0 => reqwest_middleware::ClientBuilder::new(client).build(),
        _ => reqwest_middleware::ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(
                ExponentialBackoff::builder().build_with_max_retries(max_retries),
            ))
            .build(),
    })
}

const REQUESTS_PER_STREAM: usize = 90;

impl GcpClient {
    pub fn try_new(
        api_key: String,
        maybe_referrer: Option<String>,
        max_clients: usize,
        max_retries: u32,
    ) -> Result<Self, Error> {
        if max_clients == 0 {
            return Err(Error::InvalidMaxClients);
        }

        let mut default_headers = header::HeaderMap::new();
        if let Some(referrer) = maybe_referrer {
            default_headers.insert(REFERRER, referrer.parse()?);
        }

        let clients = (0..max_clients)
            .map(|_| {
                Ok(ClientSemaphorePair {
                    client: create_client(default_headers.clone(), max_retries)?,
                    semaphore: Semaphore::new(REQUESTS_PER_STREAM),
                })
            })
            .collect::<Result<Vec<ClientSemaphorePair>, Error>>()?;

        Ok(GcpClient { api_key, clients })
    }

    pub fn try_new_from_env(max_clients: usize, max_retries: u32) -> Result<Self, Error> {
        let api_key = require_env_var_or(GOOGLE_API_KEY, Error::InvalidApiKey)?;
        let maybe_referrer = env_var(GOOGLE_REFERRER);
        Self::try_new(api_key, maybe_referrer, max_clients, max_retries)
    }
}

impl ClientSemaphorePair {
    async fn acquire(&self) -> Result<(&ClientWithMiddleware, SemaphorePermit<'_>), Error> {
        Ok((&self.client, self.semaphore.acquire().await?))
    }

    fn try_acquire(&self) -> Option<(&ClientWithMiddleware, SemaphorePermit<'_>)> {
        let maybe_permit = self.semaphore.try_acquire().ok();
        maybe_permit.map(|permit| (&self.client, permit))
    }
}

impl GcpClient {
    pub async fn next_client(&self) -> Result<(&ClientWithMiddleware, SemaphorePermit<'_>), Error> {
        if self.clients.len() < 2 {
            let first_pair = self.clients.first().expect("No GCP clients");
            first_pair.acquire().await
        } else {
            let random_pair = self
                .clients
                .iter()
                .choose(&mut rand::rng())
                .expect("No GCP clients");

            let result = random_pair.semaphore.try_acquire();
            match result {
                Ok(permit) => Ok((&random_pair.client, permit)),
                Err(TryAcquireError::NoPermits) => {
                    let maybe_next_client =
                        self.clients.iter().find_map(|client| client.try_acquire());

                    match maybe_next_client {
                        Some(next_client) => Ok(next_client),
                        None => {
                            self.clients
                                .first()
                                .expect("No GCP clients")
                                .acquire()
                                .await
                        }
                    }
                }
                Err(error) => Err(Error::Unknown(error.to_string())),
            }
        }
    }
}

const SPEECH_API_ENDPOINT: &str = "https://texttospeech.googleapis.com/v1/text:synthesize";

pub type AudioStream = Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct TextToSpeechResponse {
    #[serde(rename = "audioContent")]
    audio_content: String,
}

impl GcpClient {
    pub async fn generate_speech(
        &self,
        text: &str,
        voice: &Voice,
        audio_config: &AudioConfig,
    ) -> Result<Bytes, Error> {
        let request_json = json!({
            "input": {
                "text": text
            },
            "voice": voice,
            "audioConfig": audio_config,
        });

        let url = format!("{SPEECH_API_ENDPOINT}?key={}", self.api_key);
        let (client, _permit) = self.next_client().await?;
        let response = client.post(url).json(&request_json).send().await?;

        let status = response.status();
        match status {
            StatusCode::OK => {
                let audio_bytes = BASE64_STANDARD
                    .decode(response.json::<TextToSpeechResponse>().await?.audio_content)?;
                Ok(Bytes::from(audio_bytes))
            }
            StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimit),
            _ => Err(Error::UnexpectedApiResponse(
                format!("Expected 200 from {SPEECH_API_ENDPOINT} but got {status}").to_owned(),
            )),
        }
    }

    pub async fn generate_speech_stream(
        &self,
        _text: &str,
        _voice: &Voice,
        _audio_config: &AudioConfig,
    ) -> Result<AudioStream, Error> {
        todo!("parse stream as JSON, decode 'audioContent'");
    }
}

#[cfg(test)]
mod tests {
    use crate::audio_config::AudioConfig;
    use crate::audio_encoding::AudioEncoding;
    use crate::client::GcpClient;
    use crate::voice::Voice;
    use futures::future;

    #[tokio::test]
    #[ignore]
    async fn test_parallel_requests() {
        let client = GcpClient::try_new_from_env(1, 0).expect("Failed to create GcpClient");

        let voice = Voice {
            name: "en-US-Standard-A".to_owned(),
            language_code: "en-US".to_owned(),
        };
        let audio_config = AudioConfig {
            audio_encoding: AudioEncoding::OggOpus,
        };
        let tasks: Vec<_> = (0..105)
            .map(async |index| {
                let text = format!("{index}");
                client.generate_speech(&text, &voice, &audio_config).await
            })
            .collect();

        future::try_join_all(tasks)
            .await
            .expect("A generate_speech task failed");
    }
}
