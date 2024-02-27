use std::{env, fs};
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use once_cell::sync::Lazy;

pub const ROOT_DIR: Lazy<PathBuf> = Lazy::new(|| {
    PathBuf::new()
        .join(env::var("HOME").expect("failed to find home dir"))
        .join(".local/share")
        .join(env!("CARGO_PKG_NAME"))
});

pub const MOUNT_POINT: Lazy<PathBuf> = Lazy::new(|| {
    ROOT_DIR.join("mp")
});

pub const CONFIG_FILE: Lazy<PathBuf> = Lazy::new(|| {
    ROOT_DIR.join("patches.toml")
});


pub fn ensure_dir<P : AsRef<Path>>(dir: P) -> Result<()> {
    let dirname = dir.as_ref().to_str().unwrap().to_owned();

    if let Err(e) = fs::create_dir_all(dir) {
        bail!("failed to create dir {}: {}", dirname, e)
    }

    Ok(())
}

pub trait FileNameString {
    fn name_string(&self) -> String;
}

impl<P : AsRef<Path>> FileNameString for P {
    fn name_string(&self) -> String {
        self.as_ref().file_name().unwrap().to_str().unwrap().to_owned()
    }
}
