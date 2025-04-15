use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize_enum_str, Serialize_enum_str)]
pub enum AudioEncoding {
    #[serde(rename = "LINEAR16")]
    Linear16,
    #[serde(rename = "MP3")]
    MP3,
    #[serde(rename = "OGG_OPUS")]
    OggOpus,
    #[serde(rename = "MULAW")]
    MULAW,
    #[serde(rename = "ALAW")]
    ALAW,
    #[serde(rename = "PCM")]
    PCM,
}
