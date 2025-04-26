pub mod r2;
pub mod s3;

use crate::error::Error;
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

pub type CacheEntryStream = Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>;

#[async_trait]
pub trait CacheClient {
    async fn get(&self, key: &str) -> Result<Option<Bytes>, Error>;
    async fn stream_from(&self, key: &str) -> Result<Option<CacheEntryStream>, Error>;
    async fn store(&self, key: &str, value: Bytes) -> Result<(), Error>;
    async fn stream_into(
        &self,
        key: &str,
        stream: Pin<Box<impl Stream<Item = Result<Bytes, Error>> + Send + 'static>>,
    ) -> Result<(), Error>;
}
