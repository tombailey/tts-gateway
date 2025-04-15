use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize_enum_str, Serialize_enum_str)]
pub enum TTSModel {
    #[serde(rename = "gpt-4o-mini-tts")]
    GPT4OMiniTTS,
    #[serde(rename = "tts-1")]
    TTS1,
    #[serde(rename = "tts-1-hd")]
    TTS1HD,
}

#[cfg(test)]
mod tests {
    use crate::model::TTSModel;

    #[test]
    fn it_should_get_models_from_string() {
        assert_eq!(
            TTSModel::try_from("gpt-4o-mini".to_owned()).unwrap(),
            TTSModel::GPT4OMiniTTS
        );
        assert_eq!(
            TTSModel::try_from("tts-1".to_owned()).unwrap(),
            TTSModel::TTS1
        );
        assert_eq!(
            TTSModel::try_from("tts-1-hd".to_owned()).unwrap(),
            TTSModel::TTS1HD
        );
    }
}
