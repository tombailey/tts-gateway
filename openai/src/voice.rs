use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize_enum_str, Serialize_enum_str)]
pub enum TTSVoice {
    #[serde(rename = "alloy")]
    ALLOY,
    #[serde(rename = "ash")]
    ASH,
    #[serde(rename = "ballad")]
    BALLAD,
    #[serde(rename = "coral")]
    CORAL,
    #[serde(rename = "echo")]
    ECHO,
    #[serde(rename = "fable")]
    FABLE,
    #[serde(rename = "onyx")]
    ONYX,
    #[serde(rename = "nova")]
    NOVA,
    #[serde(rename = "sage")]
    SAGE,
    #[serde(rename = "shimmer")]
    SHIMMER,
    #[serde(rename = "verse")]
    VERSE,
}

#[cfg(test)]
mod tests {
    use crate::voice::TTSVoice;

    #[test]
    fn it_should_get_models_from_string() {
        assert_eq!(
            TTSVoice::try_from("alloy".to_owned()).unwrap(),
            TTSVoice::ALLOY
        );
        assert_eq!(TTSVoice::try_from("ash".to_owned()).unwrap(), TTSVoice::ASH);
        assert_eq!(
            TTSVoice::try_from("ballad".to_owned()).unwrap(),
            TTSVoice::BALLAD
        );
        assert_eq!(
            TTSVoice::try_from("coral".to_owned()).unwrap(),
            TTSVoice::CORAL
        );
        assert_eq!(
            TTSVoice::try_from("echo".to_owned()).unwrap(),
            TTSVoice::ECHO
        );
        assert_eq!(
            TTSVoice::try_from("fable".to_owned()).unwrap(),
            TTSVoice::FABLE
        );
        assert_eq!(
            TTSVoice::try_from("onyx".to_owned()).unwrap(),
            TTSVoice::ONYX
        );
        assert_eq!(
            TTSVoice::try_from("nova".to_owned()).unwrap(),
            TTSVoice::NOVA
        );
        assert_eq!(
            TTSVoice::try_from("sage".to_owned()).unwrap(),
            TTSVoice::SAGE
        );
        assert_eq!(
            TTSVoice::try_from("shimmer".to_owned()).unwrap(),
            TTSVoice::SHIMMER
        );
        assert_eq!(
            TTSVoice::try_from("verse".to_owned()).unwrap(),
            TTSVoice::VERSE
        );
    }
}
