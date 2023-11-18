use std::net::SocketAddr;

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Shared secret to authenticate the bridge
    pub uuid: String,
    /// Bind to this address for incoming connections
    #[arg(short = 'B', long, env = "D3XS_BIND")]
    pub bind: SocketAddr,
    /// Increase logging output (can be used multiple times)
    #[arg(short, long, global = true, action(clap::ArgAction::Count))]
    pub verbose: u8,
}
