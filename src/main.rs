#![feature(try_blocks)]

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use anyhow::{bail, Result};
use libc::{S_IFREG, S_ISUID, S_IWGRP, S_IWOTH};
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
    let mode = metadata.permissions().mode();
    
    if mode & S_IFREG == 0 {
        bail!("incorrect file type");
    }
    
    if mode & S_ISUID == 0 {
        bail!("fpatch muse be SUID file to run");
    }
    
    if mode & (S_IWGRP | S_IWOTH) != 0 {
        bail!("incorrect file permission");
    }

    if metadata.uid() != 0 {
        bail!("file owner must be root");
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
