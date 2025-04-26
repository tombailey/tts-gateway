use crate::client::s3::{DEFAULT_S3_BUCKET_PREFIX, S3CacheClient};
use crate::error::Error;
use env::{env_var, env_var_or_default, require_env_var_or};
use s3::creds::Credentials;
use s3::{Bucket, Region};

pub const R2_BUCKET_NAME: &str = "R2_BUCKET_NAME";
pub const R2_BUCKET_PREFIX: &str = "R2_BUCKET_PREFIX";
pub const R2_ACCOUNT_ID: &str = "R2_ACCOUNT_ID";
pub const R2_ACCESS_KEY_ID: &str = "R2_ACCESS_KEY_ID";
pub const R2_ACCESS_KEY_SECRET: &str = "R2_ACCESS_KEY_SECRET";
pub const R2_ENDPOINT: &str = "R2_ENDPOINT";

pub const R2_DOMAIN: &str = "r2.cloudflarestorage.com";

fn require_env_var_or_error(name: &str) -> Result<String, Error> {
    require_env_var_or(name, Error::MissingConfig(name.to_owned()))
}

impl S3CacheClient {
    pub fn try_new_r2(
        bucket_name: &str,
        bucket_prefix: String,
        account_id: &str,
        access_key_id: &str,
        access_key_secret: &str,
        maybe_endpoint: Option<&str>,
    ) -> Result<Self, Error> {
        let region = Region::Custom {
            region: "auto".to_owned(),
            endpoint: match maybe_endpoint {
                None => {
                    format!("{account_id}.{R2_DOMAIN}")
                }
                Some(endpoint) => {
                    format!("{account_id}.{endpoint}")
                }
            },
        };

        let credentials = Credentials::new(
            Some(access_key_id),
            Some(access_key_secret),
            None,
            None,
            None,
        )?;

        let bucket = Bucket::new(bucket_name, region, credentials)?.with_path_style();
        Ok(Self::new(bucket, bucket_prefix))
    }

    pub fn try_new_r2_from_env() -> Result<Self, Error> {
        let bucket_name = require_env_var_or_error(R2_BUCKET_NAME)?;
        let bucket_prefix =
            env_var_or_default(R2_BUCKET_PREFIX, DEFAULT_S3_BUCKET_PREFIX.to_owned());

        let account_id = require_env_var_or_error(R2_ACCOUNT_ID)?;
        let access_key_id = require_env_var_or_error(R2_ACCESS_KEY_ID)?;
        let access_key_secret = require_env_var_or_error(R2_ACCESS_KEY_SECRET)?;

        let maybe_endpoint = env_var(R2_ENDPOINT);

        Self::try_new_r2(
            &bucket_name,
            bucket_prefix,
            &account_id,
            &access_key_id,
            &access_key_secret,
            maybe_endpoint.as_deref(),
        )
    }
}
