use crate::client::{CacheClient, CacheEntryStream};
use crate::error::Error;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use env::{env_var, env_var_or_default, require_env_var_or};
use futures::{Stream, TryStreamExt, future};
use s3::creds::Credentials;
use s3::creds::error::CredentialsError;
use s3::error::S3Error;
use s3::{Bucket, Region};
use std::pin::Pin;
use std::str::Utf8Error;

pub const S3_BUCKET_NAME: &str = "S3_BUCKET_NAME";
pub const S3_BUCKET_PREFIX: &str = "S3_BUCKET_PREFIX";
pub(crate) const DEFAULT_S3_BUCKET_PREFIX: &str = "tts/cache";
pub const S3_REGION: &str = "S3_REGION";
pub const S3_ENDPOINT: &str = "S3_ENDPOINT";

#[derive(Clone)]
pub struct S3CacheClient {
    bucket: Box<Bucket>,
    bucket_prefix: String,
}

fn require_env_var_or_error(name: &str) -> Result<String, Error> {
    require_env_var_or(name, Error::MissingConfig(name.to_owned()))
}

impl From<S3Error> for Error {
    fn from(value: S3Error) -> Self {
        match value {
            S3Error::HttpFailWithBody(404, _) => Error::MissingObject,
            _ => Error::Unknown(value.to_string()),
        }
    }
}

impl From<CredentialsError> for Error {
    fn from(value: CredentialsError) -> Self {
        Error::Unknown(value.to_string())
    }
}

impl From<Utf8Error> for Error {
    fn from(value: Utf8Error) -> Self {
        Error::Unknown(value.to_string())
    }
}

impl S3CacheClient {
    pub fn new(bucket: Bucket, bucket_prefix: String) -> Self {
        S3CacheClient {
            bucket: Box::new(bucket),
            bucket_prefix,
        }
    }

    pub fn try_new_from_env() -> Result<Self, Error> {
        let bucket_name = require_env_var_or_error(S3_BUCKET_NAME)?;
        let bucket_prefix =
            env_var_or_default(S3_BUCKET_PREFIX, DEFAULT_S3_BUCKET_PREFIX.to_owned());

        let maybe_endpoint = env_var(S3_ENDPOINT);
        let region_string = require_env_var_or_error(S3_REGION)?;
        let region = match maybe_endpoint {
            Some(endpoint) => Region::Custom {
                region: region_string,
                endpoint,
            },
            None => region_string.parse::<Region>()?,
        };

        let credentials = Credentials::default()?;
        let bucket = Bucket::new(&bucket_name, region, credentials)?.with_path_style();
        Ok(Self::new(bucket, bucket_prefix))
    }
}

impl S3CacheClient {
    fn prefixed_key(&self, object_key: &str) -> String {
        format!("{}/{object_key}", self.bucket_prefix)
    }
}

#[async_trait]
impl CacheClient for S3CacheClient {
    async fn get(&self, key: &str) -> Result<Option<Bytes>, Error> {
        let response = self.bucket.get_object(self.prefixed_key(key)).await?;
        Ok(Some(response.bytes().clone()))
    }

    async fn stream_from(&self, key: &str) -> Result<Option<CacheEntryStream>, Error> {
        let response = self
            .bucket
            .get_object_stream(self.prefixed_key(key))
            .await?;
        Ok(Some(Box::pin(
            response
                .bytes
                .map_err(|error| Error::Unknown(error.to_string())),
        )))
    }

    async fn store(&self, key: &str, value: Bytes) -> Result<(), Error> {
        let response = self
            .bucket
            .put_object(self.prefixed_key(key), &value)
            .await?;
        let status_code = response.status_code();
        match status_code {
            200 => Ok(()),
            _ => Err(Error::UnexpectedApiResponse(format!(
                "Expected 200 when storing cache entry but got {}.",
                status_code
            ))),
        }
    }

    async fn stream_into(
        &self,
        key: &str,
        stream: Pin<Box<impl Stream<Item = Result<Bytes, Error>> + Send + 'static>>,
    ) -> Result<(), Error> {
        let bytes = stream
            .try_fold(BytesMut::new(), |mut total, new| {
                total.extend_from_slice(&new);
                future::ok(total)
            })
            .await?;
        self.store(key, bytes.freeze()).await
    }
}
