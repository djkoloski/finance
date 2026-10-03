use std::{collections::HashSet, fs::File, io::BufReader, path::Path};

use anyhow::{Result, bail};
use indexmap::IndexMap;
use regex::RegexSet;
use serde::{Deserialize, Deserializer, de};

#[derive(Deserialize)]
pub struct Config {
    pub accounts: Vec<AccountConfig>,
    pub income: GroupConfig,
    pub shared_expenses: GroupConfig,
    pub individual_expenses: GroupConfig,
    pub internal: GroupConfig,
    pub categories: IndexMap<String, CategoryConfig>,
}

impl Config {
    pub fn read(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let result = serde_json::from_reader::<_, Self>(reader)?;

        result.check()?;

        Ok(result)
    }

    fn check(&self) -> Result<()> {
        let mut categories_in_groups = HashSet::new();
        for group in [
            &self.income,
            &self.shared_expenses,
            &self.individual_expenses,
            &self.internal,
        ] {
            for include in group.include.iter() {
                if !categories_in_groups.insert(include) {
                    bail!("category '{include}' included in multiple groups");
                }
            }
        }

        for category in self.categories.keys() {
            if !categories_in_groups.contains(category) {
                bail!("category '{category}' not included in any groups");
            }
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct AccountConfig {
    #[expect(unused)]
    pub name: String,
    pub file: String,
}

#[derive(Deserialize)]
pub struct GroupConfig {
    pub include: Vec<String>,
}

#[derive(Deserialize)]
pub struct CategoryConfig {
    #[serde(deserialize_with = "compile_matchers")]
    pub matchers: RegexSet,
}

fn compile_matchers<'de, D>(deserializer: D) -> Result<RegexSet, D::Error>
where
    D: Deserializer<'de>,
{
    let patterns = <Vec<String> as Deserialize>::deserialize(deserializer)?;
    RegexSet::new(patterns).map_err(de::Error::custom)
}
