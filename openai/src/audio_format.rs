use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize_enum_str, Serialize_enum_str)]
pub enum TTSAudioFormat {
    #[serde(rename = "mp3")]
    MP3,
    #[serde(rename = "opus")]
    OPUS,
    #[serde(rename = "aac")]
    AAC,
    #[serde(rename = "flac")]
    FLAC,
    #[serde(rename = "wav")]
    WAV,
    #[serde(rename = "pcm")]
    PCM,
}

#[cfg(test)]
mod tests {
    use crate::audio_format::TTSAudioFormat;

    #[test]
    fn it_should_get_models_from_string() {
        assert_eq!(
            TTSAudioFormat::try_from("mp3".to_owned()).unwrap(),
            TTSAudioFormat::MP3
        );
        assert_eq!(
            TTSAudioFormat::try_from("opus".to_owned()).unwrap(),
            TTSAudioFormat::OPUS
        );
        assert_eq!(
            TTSAudioFormat::try_from("aac".to_owned()).unwrap(),
            TTSAudioFormat::AAC
        );
        assert_eq!(
            TTSAudioFormat::try_from("flac".to_owned()).unwrap(),
            TTSAudioFormat::FLAC
        );
        assert_eq!(
            TTSAudioFormat::try_from("wav".to_owned()).unwrap(),
            TTSAudioFormat::WAV
        );
        assert_eq!(
            TTSAudioFormat::try_from("pcm".to_owned()).unwrap(),
            TTSAudioFormat::PCM
        );
    }
}
