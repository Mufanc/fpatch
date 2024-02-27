use std::os::fd::{AsFd, OwnedFd};
use std::path::PathBuf;

use anyhow::{Context, Result};
use log::debug;
use rustix::{fs as rfs, mount, process, thread};
use rustix::fs::{CWD, Mode, OFlags};
use rustix::mount::{MountPropagationFlags, MoveMountFlags, OpenTreeFlags};
use rustix::process::{Pid, PidfdFlags};
use rustix::thread::{LinkNameSpaceType, ThreadNameSpaceType, UnshareFlags};

pub fn enter_ns(proc: Pid) -> Result<OwnedFd> {
    debug!("[{}] switch ns: {}", process::getpid().as_raw_nonzero(), proc.as_raw_nonzero());
    
    let current_link = &format!("/proc/{}/ns/mnt", process::getpid().as_raw_nonzero());
    let current_ns = rfs::open(current_link, OFlags::RDONLY, Mode::empty())?;

    let fd = process::pidfd_open(proc, PidfdFlags::empty())?;
    thread::move_into_thread_name_spaces(fd.as_fd(), ThreadNameSpaceType::MOUNT).context("failed to switch namespace")?;
    
    Ok(current_ns)
}

fn restore_ns(link: OwnedFd) -> Result<()> {
    debug!("[{}] restore ns", process::getpid().as_raw_nonzero());
    
    thread::move_into_link_name_space(link.as_fd(), Some(LinkNameSpaceType::Mount))?;
    
    Ok(())
}

pub fn bind_mount(source: &PathBuf, target: &PathBuf) -> Result<()> {
    let fd = mount::open_tree(CWD, source, OpenTreeFlags::OPEN_TREE_CLONE)?;
    let backup = enter_ns(Pid::from_raw(1).unwrap())?;
    
    mount::move_mount(fd.as_fd(), "", CWD, target, MoveMountFlags::MOVE_MOUNT_F_EMPTY_PATH)?;
    
    restore_ns(backup)?;

    Ok(())
}

pub fn unshare() -> Result<()> {
    debug!("[{}] unshare mount namespace", process::getpid().as_raw_nonzero());
    
    thread::unshare(UnshareFlags::NEWNS)?;
    mount::mount_change("/", MountPropagationFlags::PRIVATE)?;
    
    Ok(())
}
