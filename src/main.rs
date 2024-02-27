use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;

use anyhow::{bail, Result};
use rustix::path::Arg;

use crate::cli::Operation;
use crate::dirs::{FileNameString, MOUNT_POINT, ROOT_DIR};
use crate::hash::Hash;
use crate::mount::bind_mount;

mod fuse;
mod configs;
mod dirs;
mod mount;
mod cli;
mod hash;

fn check_permissions() -> Result<()> {
    let metadata = fs::metadata("/proc/self/exe")?;

    if metadata.permissions().mode() != 0o104755 {
        bail!("file permissions incorrect (expected 0o4755)");
    }

    if metadata.uid() != 0 {
        bail!("file owner should be root");
    }

    Ok(())
}

fn main() -> Result<()> {
    check_permissions()?;

    let args = cli::parse_args();

    match args.op {
        None => {
            run_server()?
        }
        Some(Operation::Mount) => {
            mount_proxies()?;
        }
    }

    Ok(())
}

fn run_server() -> Result<()> {
    env_logger::init();
    dirs::ensure_dir(ROOT_DIR.as_path())?;

    mount::unshare()?;
    fuse::mount(configs::parse())?;

    Ok(())
}

fn mount_proxies() -> Result<()> {
    let mut entries: HashMap<String, PathBuf> = HashMap::new();

    for entry in fs::read_dir(MOUNT_POINT.as_path())? {
        let path = entry.unwrap().path();

        let filename = path.name_string();
        let hash = filename.split(':').next().unwrap().to_string();

        entries.insert(hash, path);
    }

    for file in configs::parse() {
        let hash = file.path.name_string().hash();
        
        let source = &entries[&hash];
        let target = &file.path;
        
        bind_mount(source, target)?;
    }

    Ok(())
}
