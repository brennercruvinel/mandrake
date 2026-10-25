//! The catalog of Claude Code folder entries, `cfgs/setup/items.toml`, compiled into the binary.

use anyhow::{Context, Result};
use serde::Deserialize;

pub const ITEMS_TOML: &str = include_str!("../../../cfgs/setup/items.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    File,
    Dir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    Config,
    Plugins,
    Session,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Item {
    /// Path inside the Claude Code folder; also the key in the profile's `[links]`.
    pub path: String,
    pub kind: Kind,
    pub group: Group,
    #[serde(default)]
    pub sensitive: bool,
    pub about: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Catalog {
    #[serde(rename = "item")]
    pub items: Vec<Item>,
}

impl Catalog {
    pub fn builtin() -> Result<Self> {
        toml::from_str(ITEMS_TOML).context("parsing the built-in items.toml")
    }

    pub fn get(&self, path: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.path == path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_parses_and_paths_are_unique() {
        let c = Catalog::builtin().unwrap();
        assert!(c.items.len() >= 20);
        let mut paths: Vec<_> = c.items.iter().map(|i| &i.path).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), c.items.len());
        assert!(c.get("projects").unwrap().sensitive);
    }
}
