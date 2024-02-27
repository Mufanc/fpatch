use std::os::fd::AsFd;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rustix::{mount, process, thread};
use rustix::fs::CWD;
use rustix::mount::{MountPropagationFlags, MoveMountFlags, OpenTreeFlags};
use rustix::process::{Pid, PidfdFlags};
use rustix::thread::{ThreadNameSpaceType, UnshareFlags};

fn enter_ns(pid: Pid) -> Result<()> {
    let pfd = process::pidfd_open(pid, PidfdFlags::empty())?;
    
    thread::move_into_thread_name_spaces(pfd.as_fd(), ThreadNameSpaceType::MOUNT).context("failed to switch namespace")?;
    
    Ok(())
}

pub fn bind_mount(source: &PathBuf, target: &PathBuf) -> Result<()> {
    let fd = mount::open_tree(CWD, source, OpenTreeFlags::OPEN_TREE_CLONE)?;

    enter_ns(Pid::from_raw(1).unwrap())?;
    mount::move_mount(fd.as_fd(), "", CWD, target, MoveMountFlags::MOVE_MOUNT_F_EMPTY_PATH)?;
    enter_ns(process::getppid().unwrap())?;

    Ok(())
}

pub fn unshare() -> Result<()> {
    thread::unshare(UnshareFlags::NEWNS)?;
    mount::mount_change("/", MountPropagationFlags::PRIVATE)?;
    Ok(())
}
