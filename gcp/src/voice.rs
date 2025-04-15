use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct Voice {
    pub name: String,
    // TODO: use a NewType to restrict this more?
    #[serde(rename = "languageCode")]
    pub language_code: String,
}
