use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
pub struct Args {
    #[arg(short, long)]
    pub config: PathBuf,
    #[arg(short = 'B', long)]
    pub bind: SocketAddr,
    /// Increase logging output (can be used multiple times)
    #[arg(short, long, global = true, action(clap::ArgAction::Count))]
    pub verbose: u8,
}
