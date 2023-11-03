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
}

#[derive(Debug, clap::Parser)]
pub struct Open {
    pub mac: String,
    pub public_key: String,
    /// How many seconds until the operation times out (0 for no limit)
    #[arg(short, long, default_value = "15")]
    pub timeout: u64,
}
