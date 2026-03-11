use crate::audio_format::TTSAudioFormat;
use crate::error::Error;
use crate::model::TTSModel;
use crate::voice::TTSVoice;
use bytes::{Bytes, BytesMut};
use env::require_env_var_or;
use futures::{Stream, TryStreamExt, future};
use rand::prelude::IteratorRandom;
use reqwest::{StatusCode, header};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use serde_json::json;
use std::pin::Pin;
use tokio::sync::{Semaphore, SemaphorePermit, TryAcquireError};

struct ClientSemaphorePair {
    client: ClientWithMiddleware,
    semaphore: Semaphore,
}

pub struct OpenAIClient {
    clients: Vec<ClientSemaphorePair>,
}

pub const OPENAI_API_KEY: &str = "OPENAI_API_KEY";

const AUTHORIZATION: &str = "Authorization";
const BEARER: &str = "Bearer";

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

const REQUESTS_PER_STREAM: usize = 100;

impl OpenAIClient {
    pub fn try_new(api_key: String, max_clients: usize, max_retries: u32) -> Result<Self, Error> {
        if max_clients == 0 {
            return Err(Error::InvalidMaxClients);
        }

        let mut api_key_value = header::HeaderValue::try_from(format!("{BEARER} {}", api_key))?;
        api_key_value.set_sensitive(true);
        let mut default_headers = header::HeaderMap::new();
        default_headers.insert(AUTHORIZATION, api_key_value);

        let clients = (0..max_clients)
            .map(|_| {
                Ok(ClientSemaphorePair {
                    client: create_client(default_headers.clone(), max_retries)?,
                    semaphore: Semaphore::new(REQUESTS_PER_STREAM),
                })
            })
            .collect::<Result<Vec<ClientSemaphorePair>, Error>>()?;

        Ok(OpenAIClient { clients })
    }

    pub fn try_new_from_env(clients: usize, max_retries: u32) -> Result<Self, Error> {
        let api_key = require_env_var_or(OPENAI_API_KEY, Error::InvalidApiKey)?;
        Self::try_new(api_key, clients, max_retries)
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

impl OpenAIClient {
    pub async fn next_client(&self) -> Result<(&ClientWithMiddleware, SemaphorePermit<'_>), Error> {
        if self.clients.len() < 2 {
            let first_pair = self.clients.first().expect("No OpenAI clients");
            first_pair.acquire().await
        } else {
            let random_pair = self
                .clients
                .iter()
                .choose(&mut rand::rng())
                .expect("No OpenAI clients");

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
                                .expect("No OpenAI clients")
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

const SPEECH_API_ENDPOINT: &str = "https://api.openai.com/v1/audio/speech";

pub type AudioStream = Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>;

impl OpenAIClient {
    pub async fn generate_speech(
        &self,
        text: &str,
        model: &TTSModel,
        voice: &TTSVoice,
        audio_format: &TTSAudioFormat,
    ) -> Result<Bytes, Error> {
        let bytes = self
            .generate_speech_stream(text, model, voice, audio_format)
            .await?
            .try_fold(BytesMut::new(), |mut total, new| {
                total.extend_from_slice(&new);
                future::ok(total)
            })
            .await?;
        Ok(bytes.freeze())
    }

    pub async fn generate_speech_stream(
        &self,
        text: &str,
        model: &TTSModel,
        voice: &TTSVoice,
        audio_format: &TTSAudioFormat,
    ) -> Result<AudioStream, Error> {
        let request_json = json!({
            "input": text,
            "model": model,
            "voice": voice,
            "response_format": audio_format,
        });

        let (client, _permit) = self.next_client().await?;
        let response = client
            .post(SPEECH_API_ENDPOINT)
            .json(&request_json)
            .send()
            .await?;

        let status = response.status();
        match status {
            StatusCode::OK => Ok(Box::pin(
                response
                    .bytes_stream()
                    .map_err(|error| Error::Unknown(error.to_string())),
            )),
            StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimit),
            _ => Err(Error::UnexpectedApiResponse(
                format!("Expected 200 from {SPEECH_API_ENDPOINT} but got {status}").to_owned(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::audio_format::TTSAudioFormat;
    use crate::client::OpenAIClient;
    use crate::model::TTSModel;
    use crate::voice::TTSVoice;
    use futures::future;

    #[tokio::test]
    #[ignore]
    async fn test_parallel_requests() {
        let client = OpenAIClient::try_new_from_env(1, 0).expect("Failed to create OpenAIClient");

        let model = TTSModel::GPT4OMiniTTS;
        let voice = TTSVoice::NOVA;
        let audio_format = TTSAudioFormat::OPUS;
        let tasks: Vec<_> = (0..105)
            .map(async |index| {
                let text = format!("{index}");
                client
                    .generate_speech(&text, &model, &voice, &audio_format)
                    .await
            })
            .collect();

        future::try_join_all(tasks)
            .await
            .expect("A generate_speech task failed");
    }
}
