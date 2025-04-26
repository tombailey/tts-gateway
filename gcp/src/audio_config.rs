use crate::audio_encoding::AudioEncoding;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct AudioConfig {
    #[serde(rename = "audioEncoding")]
    pub audio_encoding: AudioEncoding,
}
