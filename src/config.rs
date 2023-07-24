use crate::errors::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub users: HashMap<String, User>,
    #[serde(default)]
    pub doors: HashMap<String, Door>,
}

impl Config {
    pub async fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let buf = fs::read_to_string(path).await?;
        Self::parse(&buf)
    }

    pub fn parse(buf: &str) -> Result<Self> {
        let config = toml::from_str(buf).context("Failed to load toml as config")?;
        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    #[serde(default)]
    pub authorize: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Door {
    pub label: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() -> Result<()> {
        let config = Config::parse("")?;
        assert_eq!(
            config,
            Config {
                users: HashMap::new(),
                doors: HashMap::new(),
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn parse_example() -> Result<()> {
        let config = Config::load_from_path("example.toml").await?;
        assert_eq!(
            config,
            Config {
                users: {
                    let mut m = HashMap::new();
                    m.insert(
                        "alice".to_string(),
                        User {
                            authorize: vec!["home".to_string(), "building".to_string()],
                        },
                    );
                    m.insert("bob".to_string(), User { authorize: vec![] });
                    m
                },
                doors: {
                    let mut m = HashMap::new();
                    m.insert(
                        "home".to_string(),
                        Door {
                            label: "Home".to_string(),
                        },
                    );
                    m.insert(
                        "building".to_string(),
                        Door {
                            label: "Building".to_string(),
                        },
                    );
                    m
                },
            }
        );
        Ok(())
    }
}
