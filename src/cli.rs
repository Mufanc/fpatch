use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;

use clap::Parser;

use crate::extensions::Nop;

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub op: Option<Operation>,
}

#[derive(Parser, Debug)]
pub enum Operation {
    MountFuse,
    PipeBack(PipeBackArgs)
}

pub enum OperationType {
    MountFuse,
    PipeBack
}

#[derive(Parser, Debug)]
pub struct PipeBackArgs {
    #[clap(index = 1)]
    pub pid: i32
}

pub fn parse_args() -> Args {
    Args::parse()
}

pub fn run_op(op: OperationType) -> Command {
    let mut cmd = Command::new("/proc/self/exe");

    cmd.arg0(env!("CARGO_CRATE_NAME"));

    match op {
        OperationType::MountFuse => cmd.arg("mount-fuse").nop(),
        OperationType::PipeBack => cmd.arg("pipe-back").nop()
    }

    cmd
}
