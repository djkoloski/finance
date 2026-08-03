use core::str::FromStr as _;
use std::{fs::File, io::BufReader, path::Path};

use anyhow::Result;
use indexmap::IndexMap;
use regex::RegexSet;
use serde::{Deserialize, Deserializer, de};
use temporal_rs::PlainDate;

#[derive(Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "date_string")]
    pub end_date: PlainDate,
    pub accounts: Vec<AccountConfig>,
    pub investments: InvestmentsConfig,
    pub income: GroupConfig,
    pub shared_expenses: GroupConfig,
    pub individual_expenses: GroupConfig,
    pub categories: IndexMap<String, CategoryConfig>,
}

impl Config {
    pub fn read(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }
}

#[derive(Deserialize)]
pub struct AccountConfig {
    #[expect(unused)]
    pub name: String,
    pub file: String,
}

#[derive(Deserialize)]
pub struct InvestmentsConfig {
    pub brokerage: f64,
    pub retirement: f64,
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

fn date_string<'de, D>(deserializer: D) -> Result<PlainDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <String as Deserialize>::deserialize(deserializer)?;
    PlainDate::from_str(&s).map_err(de::Error::custom)
}

fn compile_matchers<'de, D>(deserializer: D) -> Result<RegexSet, D::Error>
where
    D: Deserializer<'de>,
{
    let patterns = <Vec<String> as Deserialize>::deserialize(deserializer)?;
    RegexSet::new(patterns).map_err(de::Error::custom)
}
