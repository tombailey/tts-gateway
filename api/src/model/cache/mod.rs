use bytes::Bytes;
use cache::client::s3::S3CacheClient;
use cache::client::{CacheClient, CacheEntryStream};
use futures::Stream;
use gcp::audio_encoding::AudioEncoding;
use gcp::voice::Voice;
use openai::audio_format::TTSAudioFormat;
use openai::model::TTSModel;
use openai::voice::TTSVoice;
use std::pin::Pin;
use tracing::log::warn;

pub(crate) fn create_gcp_cache_key(
    text: &str,
    voice: &Voice,
    audio_encoding: &AudioEncoding,
) -> String {
    let text_hash = blake3::hash(text.as_bytes()).to_hex();
    format!(
        "{}/{}/{audio_encoding}/{}",
        voice.name, voice.language_code, text_hash
    )
}

pub(crate) fn create_openai_cache_key(
    text: &str,
    model: &TTSModel,
    voice: &TTSVoice,
    audio_format: &TTSAudioFormat,
) -> String {
    let text_hash = blake3::hash(text.as_bytes()).to_hex();
    format!("{model}/{voice}/{audio_format}/{}", text_hash)
}

pub(crate) async fn stream_from_cache(
    cache_client: &S3CacheClient,
    key: &str,
) -> Option<CacheEntryStream> {
    cache_client
        .stream_from(key)
        .await
        .inspect_err(|cache_error| match cache_error {
            cache::error::Error::MissingObject => {}
            _ => warn!("Cache output error: {}", cache_error),
        })
        .unwrap_or(None)
}

pub(crate) async fn store_in_cache(cache_client: &S3CacheClient, key: &str, bytes: Bytes) {
    if let Err(cache_error) = cache_client.store(key, bytes).await {
        warn!("Cache input error: {}", cache_error);
    }
}

pub(crate) async fn stream_into_cache(
    cache_client: &S3CacheClient,
    key: &str,
    stream: Pin<Box<impl Stream<Item = Result<Bytes, cache::error::Error>> + Send + 'static>>,
) {
    if let Err(cache_error) = cache_client.stream_into(key, stream).await {
        warn!("Cache input error: {}", cache_error);
    }
}
