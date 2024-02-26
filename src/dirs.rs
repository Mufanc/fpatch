use std::{env, fs};
use std::path::{Path, PathBuf};
use log::debug;
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


pub fn ensure_dir<P : AsRef<Path>>(dir: P) {
    let dirname = dir.as_ref().to_str().unwrap().to_owned();
    fs::create_dir_all(dir).unwrap_or_else(|err| panic!("failed to create dir {}: {}", dirname, err));
}
