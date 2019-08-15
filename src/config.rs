use crate::model::{PartOfSpeech,Release};
use std::collections::HashMap;

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub pos_property : Option<String>,
    pub pos_mapping : Option<HashMap<String, PartOfSpeech>>,
    pub default_release : Option<Release>,
    pub default_id : Option<String>
}

impl Config {
    pub fn blank() -> Config {
        Config {
            pos_property: None,
            pos_mapping: None,
            default_release: None,
            default_id: None
        }
    }
}
