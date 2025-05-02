use crate::AppState;
use crate::model::cache::{
    create_gcp_cache_key, create_openai_cache_key, store_in_cache, stream_from_cache,
    stream_into_cache,
};
use crate::model::stream::duplicate;
use actix_web::{HttpResponse, ResponseError, post, web};
use cache::client::s3::S3CacheClient;
use futures::TryStreamExt;
use gcp::audio_config::AudioConfig;
use gcp::audio_encoding::AudioEncoding;
use gcp::voice::Voice;
use openai::audio_format::TTSAudioFormat;
use openai::model::TTSModel;
use openai::voice::TTSVoice;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::log::error;
use tracing_futures::Instrument;

#[derive(Error, Debug)]
pub enum SpeechRouteError {
    #[error("GcpError: {0}")]
    GcpError(#[from] gcp::error::Error),
    #[error("OpenAIError: {0}")]
    OpenAIError(#[from] openai::error::Error),
    #[error("UnsupportedVendor: {0}")]
    UnsupportedVendor(String),
}

impl ResponseError for SpeechRouteError {
    fn error_response(&self) -> HttpResponse {
        error!("{}", self);
        HttpResponse::InternalServerError().finish()
    }
}

trait ContentType {
    fn content_type(&self) -> &'static str;
}

impl ContentType for TTSAudioFormat {
    fn content_type(&self) -> &'static str {
        match self {
            TTSAudioFormat::MP3 => "audio/mpeg",
            TTSAudioFormat::OPUS => "audio/ogg",
            TTSAudioFormat::AAC => "audio/aac",
            TTSAudioFormat::FLAC => "audio/flac",
            TTSAudioFormat::WAV => "audio/wav",
            TTSAudioFormat::PCM => "application/octet-stream",
        }
    }
}

const WAS_CACHED_HEADER: &str = "X-WAS-CACHED";
const TRUE_STR: &str = "true";
const FALSE_STR: &str = "false";

struct KeyCacheClient {
    key: String,
    cache_client: S3CacheClient,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
struct GcpRequest {
    text: String,
    voice: Voice,
    #[serde(rename = "audioConfig")]
    audio_config: AudioConfig,
}

impl ContentType for AudioEncoding {
    fn content_type(&self) -> &'static str {
        match self {
            AudioEncoding::MULAW | AudioEncoding::ALAW | AudioEncoding::Linear16 => "audio/wav",
            AudioEncoding::MP3 => "audio/mpeg",
            AudioEncoding::OggOpus => "audio/ogg",
            AudioEncoding::PCM => "application/octet-stream",
        }
    }
}

#[post("/v1/speech/gcp")]
pub async fn gcp_speech(
    (request, app_state): (web::Json<GcpRequest>, web::Data<AppState>),
) -> Result<HttpResponse, SpeechRouteError> {
    let audio_format = &request.audio_config.audio_encoding;
    let maybe_key_cache_client = {
        app_state.maybe_cache_client.as_ref().map(|cache_client| {
            let key = create_gcp_cache_key(&request.text, &request.voice, &audio_format);
            KeyCacheClient {
                key,
                cache_client: cache_client.clone(),
            }
        })
    };

    let maybe_cache_entry_stream = match maybe_key_cache_client.as_ref() {
        Some(KeyCacheClient { key, cache_client }) => {
            stream_from_cache(cache_client, key)
                .instrument(tracing::info_span!("gcp-speech-cache-lookup"))
                .await
        }
        None => None,
    };

    if let Some(cached_stream) = maybe_cache_entry_stream {
        Ok(HttpResponse::Ok()
            .content_type(request.audio_config.audio_encoding.content_type())
            .append_header((WAS_CACHED_HEADER, TRUE_STR))
            .streaming(
                cached_stream
                    .instrument(tracing::info_span!("gcp-speech-cache-retrieval"))
                    .into_inner(),
            ))
    } else {
        if let Some(gcp_client) = &app_state.maybe_gcp_client {
            // TODO: support streaming
            let audio_bytes = gcp_client
                .generate_speech(&request.text, &request.voice, &request.audio_config)
                .instrument(tracing::info_span!("gcp-speech-generation"))
                .await?;

            if let Some(KeyCacheClient { key, cache_client }) = maybe_key_cache_client {
                let audio_bytes_to_cache = audio_bytes.clone();
                actix_web::rt::spawn(async move {
                    store_in_cache(&cache_client, &key, audio_bytes_to_cache)
                        .instrument(tracing::info_span!("gcp-speech-cache-store"))
                        .await;
                });
            }

            Ok(HttpResponse::Ok()
                .content_type(audio_format.content_type())
                .append_header((WAS_CACHED_HEADER, FALSE_STR))
                .body(audio_bytes))
        } else {
            Err(SpeechRouteError::UnsupportedVendor(
                "GCP speech is not supported".to_owned(),
            ))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
enum OpenAIResponseFormat {}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
struct OpenAIRequest {
    text: String,
    model: TTSModel,
    voice: TTSVoice,
    #[serde(rename = "responseFormat")]
    response_format: Option<TTSAudioFormat>,
}

#[post("/v1/speech/openai")]
pub async fn openai_speech(
    (request, app_state): (web::Json<OpenAIRequest>, web::Data<AppState>),
) -> Result<HttpResponse, SpeechRouteError> {
    let audio_format = request
        .response_format
        .as_ref()
        .unwrap_or(&TTSAudioFormat::MP3);

    let maybe_key_cache_client = {
        app_state.maybe_cache_client.as_ref().map(|cache_client| {
            let key = create_openai_cache_key(
                &request.text,
                &request.model,
                &request.voice,
                &audio_format,
            );
            KeyCacheClient {
                key,
                cache_client: cache_client.clone(),
            }
        })
    };

    let maybe_cache_entry_stream = match maybe_key_cache_client.as_ref() {
        Some(KeyCacheClient { key, cache_client }) => {
            stream_from_cache(cache_client, key)
                .instrument(tracing::info_span!("openai-speech-cache-lookup"))
                .await
        }
        None => None,
    };

    if let Some(cached_stream) = maybe_cache_entry_stream {
        Ok(HttpResponse::Ok()
            .content_type(audio_format.content_type())
            .append_header((WAS_CACHED_HEADER, TRUE_STR))
            .streaming(
                cached_stream
                    .instrument(tracing::info_span!("openai-speech-cache-retrieval"))
                    .into_inner(),
            ))
    } else {
        if let Some(openai_client) = &app_state.maybe_openai_client {
            let original_audio_stream = openai_client
                .generate_speech_stream(
                    &request.text,
                    &request.model,
                    &request.voice,
                    &audio_format,
                )
                .await?;

            let audio_stream =
                match maybe_key_cache_client {
                    None => original_audio_stream,
                    Some(KeyCacheClient { key, cache_client }) => {
                        let (audio_stream, audio_stream_to_cache) =
                            duplicate(original_audio_stream, 50_usize);
                        actix_web::rt::spawn(async move {
                            stream_into_cache(
                                &cache_client,
                                &key,
                                Box::pin(audio_stream_to_cache.map_err(|error| {
                                    cache::error::Error::Unknown(error.to_string())
                                })),
                            )
                            .instrument(tracing::info_span!("openai-speech-cache-store"))
                            .await;
                        });
                        Box::pin(audio_stream)
                    }
                };

            Ok(HttpResponse::Ok()
                .content_type(audio_format.content_type())
                .append_header((WAS_CACHED_HEADER, FALSE_STR))
                .streaming(
                    audio_stream
                        .instrument(tracing::info_span!("openai-speech-generation"))
                        .into_inner(),
                ))
        } else {
            Err(SpeechRouteError::UnsupportedVendor(
                "OpenAI speech is not supported".to_owned(),
            ))
        }
    }
}
