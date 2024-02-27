use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::dirs::CONFIG_FILE;

#[derive(Deserialize, Debug)]
struct PatchModel {
    file: String,
    content: String
}

#[derive(Deserialize, Debug)]
struct PatchConfigsModel {
    prepend: Option<Vec<PatchModel>>,
    append: Option<Vec<PatchModel>>,
    replace: Option<Vec<PatchModel>>
}

#[derive(Debug, Copy, Clone)]
pub enum PatchType {
    Prepend,
    Append,
    Replace
}

#[derive(Debug)]
pub struct PatchedFile {
    pub patch_type: PatchType,
    pub path: PathBuf,
    pub content: Vec<u8>
}

pub fn parse() -> Vec<PatchedFile> {
    let configs_str = fs::read_to_string(&*CONFIG_FILE).expect("failed to read config file");
    let configs: PatchConfigsModel = toml::from_str(&configs_str).expect("failed to parse configs");

    let mut patches = vec![];
    let mut transform = |ty: PatchType, models: Vec<PatchModel>| {
        models.into_iter().for_each(|model| {
            patches.push(PatchedFile {
                patch_type: ty,
                path: PathBuf::from(&model.file),
                content: model.content.into()
            });
        });
    };

    if let Some(models) = configs.prepend {
        transform(PatchType::Prepend, models);
    }

    if let Some(models) = configs.append {
        transform(PatchType::Append, models);
    }

    if let Some(models) = configs.replace {
        transform(PatchType::Replace, models);
    }

    return patches
}
