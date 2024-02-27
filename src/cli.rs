use clap::Parser;
use tokio::process::Command;

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub op: Option<Operation>,
}

#[derive(Parser, Debug)]
pub enum Operation {
    MountFuse,
    PipeBack
}

pub fn parse_args() -> Args {
    Args::parse()
}

pub fn run_self() -> Command {
    Command::new("/proc/self/exe")
}
