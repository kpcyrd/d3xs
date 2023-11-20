pub mod args;
pub mod ble;
pub mod config;
pub mod errors;
pub mod ws;

use crate::args::{Args, SubCommand};
use crate::errors::*;
use clap::Parser;
use d3xs_protocol::chall;
use d3xs_protocol::crypto;
use data_encoding::{BASE64, BASE64URL_NOPAD};
use env_logger::Env;
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::time;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => "info",
        _ => "debug",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    match args.subcommand {
        SubCommand::Open(open) => {
            let public_key = crypto::public_key(&open.public_key)
                .map_err(|_| anyhow!("Failed to parse public key"))?;
            let secret_key = crypto::secret_key(&open.secret_key)
                .map_err(|_| anyhow!("Failed to parse secret key"))?;
            let salsa = crypto::SalsaBox::new(&public_key, &secret_key);
            ble::open(&salsa, &open.mac, open.timeout).await?
        }
        SubCommand::Connect(connect) => {
            let config = config::Config::load_from_path(connect.config).await?;
            let mut challenges = chall::UserDoorMap::default();

            let url = if let Some(url) = &connect.url {
                url
            } else if let Some(url) = &config.system.url {
                url
            } else {
                bail!("Missing url to connect to");
            };

            loop {
                if let Err(err) = ws::connect(url, &config, &mut challenges).await {
                    error!("Websocket error: {err:#}");
                }
                time::sleep(time::Duration::from_secs(3)).await;
                info!("Reconnecting...");
            }
        }
        SubCommand::Keygen(keygen) => {
            let secret_key = if keygen.stdin {
                let mut stdin = io::stdin();
                let mut buf = Vec::new();
                stdin.read_to_end(&mut buf).await?;
                let buf = buf.strip_suffix(b"\n").unwrap_or(&buf);

                let buf = BASE64.decode(buf)?;
                if buf.len() != crypto::CRYPTO_SECRET_KEY_SIZE {
                    bail!("Unexpected length for secret key: {}", buf.len());
                }

                let mut key = [0u8; crypto::CRYPTO_SECRET_KEY_SIZE];
                key.copy_from_slice(&buf);
                crypto::SecretKey::from(key)
            } else {
                crypto::generate_secret_key::<crypto::Random>()
            };
            let public_key = secret_key.public_key();

            let secret_key = BASE64.encode(&secret_key.to_bytes());
            let path = BASE64URL_NOPAD.encode(public_key.as_bytes());
            let public_key = BASE64.encode(public_key.as_bytes());

            let url = keygen.url.as_deref().unwrap_or("https://example.com");
            let url: &str = url.strip_suffix('/').unwrap_or(url);

            if keygen.bridge {
                println!("[system]");
                println!("# public_key = {public_key:?}");
                println!("secret_key = {secret_key:?}");
            } else if keygen.firmware {
                println!("# [doors.building]");
                println!("# label = \"Building\"");
                println!("# mac = \"ec:da:3b:ff:ff:ff\"");
                println!("# public_key = {public_key:?}");
                println!("D3XS_DOOR_KEY={secret_key:?}");
            } else {
                let name = keygen.name.as_deref();
                let path = if keygen.name_as_path {
                    name.context("Name was not provided, can't use as path")?
                } else {
                    &path
                };
                println!("[users.{}]", name.unwrap_or("alice"));
                println!("# {url}/{path}#{secret_key}");
                println!("public_key = {public_key:?}");
                println!("authorize = []");
            }
        }
    }

    Ok(())
}
