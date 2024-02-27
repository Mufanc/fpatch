#![feature(try_blocks)]

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use anyhow::{bail, Result};
use tokio::runtime::Runtime;

use crate::cli::Operation;
use crate::dirs::ROOT_DIR;

mod fuse;
mod configs;
mod dirs;
mod mount;
mod cli;
mod hash;
mod daemon;
mod extensions;
mod pipeback;

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
    env_logger::init();

    check_permissions()?;
    dirs::ensure_dir(&*ROOT_DIR)?;

    let args = cli::parse_args();

    match args.op {
        None => {
            let runtime = Runtime::new()?;
            runtime.block_on(daemon::main())?;
        }
        Some(Operation::MountFuse) => {
            mount::unshare()?;
            fuse::mount(configs::parse())?;
        },
        Some(Operation::PipeBack(args)) => {
            pipeback::main(args.pid)?;
        }
    }

    Ok(())
}
