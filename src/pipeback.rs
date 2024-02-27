use anyhow::Result;
use rustix::process::Pid;

use crate::mount;

pub fn main(pid: i32) -> Result<()> {
    mount::pipe_back(Pid::from_raw(pid).unwrap())?;
    Ok(())
}
