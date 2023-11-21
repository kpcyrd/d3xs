use crate::errors::*;
use d3xs_protocol::crypto;
use d3xs_protocol::ipc;
use data_encoding::BASE64;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub system: Bridge,
    #[serde(default)]
    pub users: HashMap<String, User>,
    #[serde(default)]
    pub doors: HashMap<String, Door>,
}

impl Config {
    pub async fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let buf = fs::read_to_string(path)
            .await
            .with_context(|| anyhow!("Failed to load config from {path:?}"))?;
        Self::parse(&buf)
    }

    pub fn parse(buf: &str) -> Result<Self> {
        let config = toml::from_str(buf).context("Failed to load toml as config")?;
        Ok(config)
    }

    pub fn to_shared_config(&self) -> Result<ipc::Config> {
        let secret_key = crypto::secret_key(&self.system.secret_key)
            .ok()
            .context("Failed to decode secret key")?;
        let public_key = secret_key.public_key();
        let public_key = BASE64.encode(public_key.as_bytes());

        let users = self
            .users
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    ipc::User {
                        authorize: v.authorize.clone(),
                    },
                )
            })
            .collect();
        let doors = self
            .doors
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    ipc::Door {
                        label: v.label.clone(),
                    },
                )
            })
            .collect();
        Ok(ipc::Config {
            public_key,
            users,
            doors,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bridge {
    pub secret_key: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub public_key: String,
    #[serde(default)]
    pub authorize: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Door {
    pub label: String,
    pub mac: Option<String>,
    pub public_key: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() -> Result<()> {
        let config = Config::parse(
            r#"[system]
secret_key = "cRbNcnt4bw49I/AQb0wcjIBoqoLBayZAneTCGuG1g9g="
"#,
        )?;
        assert_eq!(
            config,
            Config {
                system: Bridge {
                    secret_key: "cRbNcnt4bw49I/AQb0wcjIBoqoLBayZAneTCGuG1g9g=".to_string(),
                    url: None,
                },
                users: HashMap::new(),
                doors: HashMap::new(),
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn parse_example() -> Result<()> {
        let config = Config::load_from_path("../example.toml").await?;
        assert_eq!(
            config,
            Config {
                system: Bridge {
                    secret_key: "cRbNcnt4bw49I/AQb0wcjIBoqoLBayZAneTCGuG1g9g=".to_string(),
                    url: None,
                },
                users: {
                    let mut m = HashMap::new();
                    m.insert(
                        "alice".to_string(),
                        User {
                            public_key: "TpR3WQMpINCjZoLqAtNQcAZxwIqcITji+8KLJfdJEFc=".to_string(),
                            authorize: vec!["home".to_string(), "building".to_string()],
                        },
                    );
                    m.insert(
                        "bob".to_string(),
                        User {
                            public_key: "7Pb0/x8UgjvcInZFy8FX+o/8pgMQHc2G42BftKnsBUo=".to_string(),
                            authorize: vec![],
                        },
                    );
                    m
                },
                doors: {
                    let mut m = HashMap::new();
                    m.insert(
                        "home".to_string(),
                        Door {
                            label: "Home".to_string(),
                            mac: None,
                            public_key: None,
                        },
                    );
                    m.insert(
                        "building".to_string(),
                        Door {
                            label: "Building".to_string(),
                            mac: Some("ec:da:3b:ff:ff:ff".to_string()),
                            public_key: Some(
                                "6JgMhuAy8espdQUujWW93RXDtZZBF07JZ4pTeJ2Sx1Q=".to_string(),
                            ),
                        },
                    );
                    m
                },
            }
        );
        Ok(())
    }
}
