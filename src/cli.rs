use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub op: Option<Operation>,
}

#[derive(Parser, Debug)]
pub enum Operation {
    Mount
}

pub fn parse_args() -> Args {
    Args::parse()
}
