#![feature(try_blocks)]

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use anyhow::{bail, Result};
use log::debug;

use crate::cli::Operation;
use crate::dirs::ROOT_DIR;

mod fuse;
mod configs;
mod dirs;
mod mount;
mod cli;
mod hash;
mod daemon;

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

fn check_ns() -> Result<()> {
    let ns = fs::read_link("/proc/thread-self/ns/mnt")?;
    debug!("current namespace: {ns:?}");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    check_permissions()?;
    
    dirs::ensure_dir(ROOT_DIR.as_path())?;

    let args = cli::parse_args();

    match args.op {
        None => {
            mount::unshare()?;
            cli::run_self().arg("daemon").status().await?;
        }
        Some(Operation::MountFuse) => {
            fuse::mount(configs::parse())?;
        }
        Some(Operation::Daemon) => {
            daemon::main().await?;
        }
    }

    Ok(())
}
