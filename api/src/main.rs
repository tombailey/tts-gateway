#![feature(trait_alias)]

mod model;
mod router;

use crate::router::speech::gcp_speech;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, web};
use cache::client::CacheClient;
use cache::client::s3::S3CacheClient;
use env::{env_var_or_default, require_env_var};
use gcp::client::GcpClient;
use openai::client::OpenAIClient;
use router::health::get_health;
use router::speech::openai_speech;
use tracing::Level;
use tracing::level_filters::LevelFilter;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, fmt};

pub trait AppStateCacheClient = CacheClient + Send + Sync;

pub const ENABLE_CACHING: &str = "ENABLE_CACHING";

pub struct AppState {
    maybe_cache_client: Option<S3CacheClient>,
    maybe_gcp_client: Option<GcpClient>,
    maybe_openai_client: Option<OpenAIClient>,
}

fn maybe_create_cache_client() -> Result<Option<S3CacheClient>, cache::error::Error> {
    let enable_caching =
        env_var_or_default(ENABLE_CACHING, "true".to_owned()).eq_ignore_ascii_case("true");
    if enable_caching {
        S3CacheClient::try_new_r2_from_env()
            .or(S3CacheClient::try_new_from_env())
            .map(Some)
    } else {
        Ok(None)
    }
}

fn maybe_create_gcp_client(
    max_clients: usize,
    max_retries: u32,
) -> Result<GcpClient, gcp::error::Error> {
    GcpClient::try_new_from_env(max_clients, max_retries)
}

fn maybe_create_openai_client(
    max_clients: usize,
    max_retries: u32,
) -> Result<OpenAIClient, openai::error::Error> {
    OpenAIClient::try_new_from_env(max_clients, max_retries)
}

fn setup_tracing() {
    let desired_level = Level::INFO;
    let filter_layer = LevelFilter::from(desired_level);
    let json_tracing = fmt::layer()
        .json()
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(filter_fn(move |metadata| {
            metadata.level().eq(&desired_level)
                && (metadata.name().contains("-speech-cache")
                    || metadata.name().contains("-speech-generation")
                    || metadata.name() == "HTTP request")
        }));
    let text_tracing = fmt::layer().compact();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(json_tracing)
        .with(text_tracing)
        .init();
}

#[actix_web::main]
async fn main() {
    setup_tracing();

    let port = require_env_var("PORT")
        .expect("Missing port.")
        .parse::<u16>()
        .expect("Invalid port.");

    let maybe_cache_client = maybe_create_cache_client().expect("Failed to create CacheClient");

    // TODO: make max_clients/max_retries env vars
    let maybe_gcp_client = maybe_create_gcp_client(10, 0).ok();
    let maybe_openai_client = maybe_create_openai_client(10, 0).ok();
    let app_data = web::Data::new(AppState {
        maybe_cache_client,
        maybe_gcp_client,
        maybe_openai_client,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .wrap(Logger::default())
            .wrap(TracingLogger::default())
            .service(get_health)
            .service(gcp_speech)
            .service(openai_speech)
    })
    .bind(("0.0.0.0", port))
    .expect("Failed to start server")
    .run()
    .await
    .expect("Failed to start server");
}
