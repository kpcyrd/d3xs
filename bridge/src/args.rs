use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Increase logging output (can be used multiple times)
    #[arg(short, long, global = true, action(clap::ArgAction::Count))]
    pub verbose: u8,
    #[command(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum SubCommand {
    Open(Open),
    Connect(Connect),
    Keygen(Keygen),
}

/// Connect to a door and open it
#[derive(Debug, clap::Parser)]
pub struct Open {
    pub mac: String,
    pub public_key: String,
    pub secret_key: String,
    /// How many seconds until the operation times out (0 for no limit)
    #[arg(short, long, default_value = "15")]
    pub timeout: u64,
}

/// Connect to a d3xs websocket server
#[derive(Debug, clap::Parser)]
pub struct Connect {
    /// Address of websocket to connect to (including uuid)
    pub url: String,
    #[arg(short, long, env = "D3XS_CONFIG")]
    pub config: PathBuf,
    /// How many seconds until the bluetooth operation times out (0 for no limit)
    #[arg(short, long, default_value = "15")]
    pub timeout: u64,
}

/// Generate a keypair
#[derive(Debug, clap::Parser)]
pub struct Keygen {
    #[arg(long)]
    pub url: Option<String>,
    /// Select a name instead of the default one (does not apply to --bridge or --firmware)
    #[arg(short = 'n', long)]
    pub name: Option<String>,
    /// Use a custom string for the path instead of deriving the public key
    #[arg(short = 'p')]
    pub name_as_path: bool,
    /// Generate a bridge key
    #[arg(long)]
    pub bridge: bool,
    /// Generate a door key
    #[arg(long)]
    pub firmware: bool,
    /// Read secret key from stdin instead of generating
    #[arg(long)]
    pub stdin: bool,
}
