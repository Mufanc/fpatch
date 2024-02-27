use std::fs;
use std::os::fd::AsFd;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use log::debug;
use rustix::{mount, process, thread};
use rustix::fs::CWD;
use rustix::mount::{MountPropagationFlags, MoveMountFlags, OpenTreeFlags, UnmountFlags};
use rustix::process::Pid;
use rustix::thread::UnshareFlags;
use tokio::time;

use crate::dirs::MOUNT_POINT;
use crate::fuse::ROOT_INO;

pub async fn take_fuse(child: Pid) -> Result<()> {
    let relative: PathBuf = MOUNT_POINT.components().skip(1).collect();
    let magic = PathBuf::from(format!("/proc/{}/root", child.as_raw_nonzero())).join(relative);

    while fs::metadata(&magic)?.ino() != ROOT_INO {
        debug!("{:?} is not fuse, wait 1 second and retry...", magic);
        time::sleep(Duration::from_secs(1)).await;
    } 

    let fd = mount::open_tree(CWD, magic, OpenTreeFlags::OPEN_TREE_CLONE)?;

    mount::move_mount(fd.as_fd(), "", CWD, &*MOUNT_POINT, MoveMountFlags::MOVE_MOUNT_F_EMPTY_PATH)?;

    debug!("copy fuse from: {}", child.as_raw_nonzero());

    Ok(())
}

pub fn bind_mount(source: &PathBuf, target: &PathBuf) -> Result<()> {
    let fd = mount::open_tree(CWD, source, OpenTreeFlags::OPEN_TREE_CLONE)?;

    mount::move_mount(fd.as_fd(), "", CWD, target, MoveMountFlags::MOVE_MOUNT_F_EMPTY_PATH)?;

    debug!("bind mount: {:?} -> {:?}", source, target);

    Ok(())
}

pub fn cleanup() -> Result<()> {
    let mounts = fs::read_to_string("/proc/self/mounts")?;

    for line in mounts.split('\n') {
        if line.is_empty() {
            break
        }

        let mut splits = line.split_ascii_whitespace();

        let fs = splits.next().unwrap();
        let mp = splits.next().unwrap();

        if fs == env!("CARGO_CRATE_NAME") {
            mount::unmount(mp, UnmountFlags::DETACH)?;
            debug!("unmount: {}", mp);
        }
    }

    Ok(())
}

pub fn unshare() -> Result<()> {
    debug!("[{}] unshare mount namespace", process::getpid().as_raw_nonzero());

    thread::unshare(UnshareFlags::NEWNS)?;
    mount::mount_change("/", MountPropagationFlags::PRIVATE)?;

    Ok(())
}
