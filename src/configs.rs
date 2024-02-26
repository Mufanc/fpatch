use std::fs;

use serde::Deserialize;

use crate::dirs::CONFIG_FILE;

#[derive(Deserialize, Debug)]
pub struct PatchModel {
    pub file: String,
    pub content: String
}

#[derive(Deserialize, Debug)]
pub struct PatchConfigsModel {
    pub prepend: Option<Vec<PatchModel>>,
    pub append: Option<Vec<PatchModel>>,
    pub replace: Option<Vec<PatchModel>>
}

pub fn parse() -> PatchConfigsModel {
    let configs_str = fs::read_to_string(CONFIG_FILE.as_path()).expect("failed to read config file");
    let configs: PatchConfigsModel = toml::from_str(&configs_str).expect("failed to parse configs");
    
    return configs
}
