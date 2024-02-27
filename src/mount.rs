use std::fs;
use std::os::fd::{AsFd, OwnedFd};
use std::path::PathBuf;

use anyhow::{Context, Result};
use log::debug;
use rustix::{mount, process, thread};
use rustix::fs::{CWD, Mode, OFlags};
use rustix::fs as rfs;
use rustix::mount::{MountPropagationFlags, MoveMountFlags, OpenTreeFlags, UnmountFlags};
use rustix::process::{Pid, PidfdFlags};
use rustix::thread::{LinkNameSpaceType, ThreadNameSpaceType, UnshareFlags};

use crate::dirs::MOUNT_POINT;

pub fn unshare() -> Result<()> {
    debug!("{}: unshare mount namespace", process::getpid().as_raw_nonzero());

    thread::unshare(UnshareFlags::NEWNS)?;
    mount::mount_change("/", MountPropagationFlags::PRIVATE)?;

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

pub fn switch_ns(pid: Pid) -> Result<OwnedFd> {
    debug!("switch ns: {} -> {}", process::getpid().as_raw_nonzero(), pid.as_raw_nonzero());

    let current_link = &format!("/proc/{}/ns/mnt", process::getpid().as_raw_nonzero());
    let current_ns = rfs::open(current_link, OFlags::RDONLY, Mode::empty())?;

    let fd = process::pidfd_open(pid, PidfdFlags::empty())?;
    thread::move_into_thread_name_spaces(fd.as_fd(), ThreadNameSpaceType::MOUNT).context("failed to switch namespace")?;

    Ok(current_ns)
}

fn restore_ns(link: OwnedFd) -> Result<()> {
    debug!("restore ns: {}", process::getpid().as_raw_nonzero());

    thread::move_into_link_name_space(link.as_fd(), Some(LinkNameSpaceType::Mount))?;

    Ok(())
}

pub fn pipe_back(pid: Pid) -> Result<()> {
    let backup = switch_ns(pid)?;
    let fd = mount::open_tree(CWD, &*MOUNT_POINT, OpenTreeFlags::OPEN_TREE_CLONE)?;

    restore_ns(backup)?;
    mount::move_mount(fd.as_fd(), "", CWD, &*MOUNT_POINT, MoveMountFlags::MOVE_MOUNT_F_EMPTY_PATH)?;

    Ok(())
}
