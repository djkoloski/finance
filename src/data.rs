use core::str::FromStr as _;
use std::{fs::File, io::BufReader, path::Path};

use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Deserializer, de};
use temporal_rs::PlainDate;

use crate::{config::Config, dollars::Dollars};

pub struct Data {
    pub meta: Meta,
    pub accounts: Vec<Account>,
}

impl Data {
    pub fn read(config: &Config, data_path: &Path) -> Result<Self> {
        let meta_path = data_path.join("meta.json");
        let meta = Meta::read(&meta_path)?;

        let mut accounts = Vec::new();

        for account_config in config.accounts.iter() {
            let mut transactions = Vec::new();

            let path = data_path.join(&account_config.file);
            let mut reader = csv::Reader::from_reader(File::open(&path)?);
            for result in reader.records() {
                let record = result?;

                let transaction = Transaction {
                    date: PlainDate::from_str(&record[0])?,
                    amount: Dollars::from_f64(record[2].parse::<f64>()?),
                    kind: match &record[3] {
                        "Deposit" => Kind::Deposit,
                        "Withdrawal" => Kind::Withdrawal,
                        _ => bail!("invalid transaction kind '{}'", &record[3]),
                    },
                    description: record[4].to_string(),
                    category: Category::from_description(config, &record[4])?,
                };

                transactions.push(transaction);
            }

            accounts.push(Account { transactions });
        }

        Ok(Self { meta, accounts })
    }
}

#[derive(Deserialize)]
pub struct Meta {
    #[serde(deserialize_with = "date_string")]
    pub end_date: PlainDate,
    pub investments: Investments,
}

impl Meta {
    pub fn read(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }
}

fn date_string<'de, D>(deserializer: D) -> Result<PlainDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <String as Deserialize>::deserialize(deserializer)?;
    PlainDate::from_str(&s).map_err(de::Error::custom)
}

#[derive(Deserialize)]
pub struct Investments {
    pub brokerage: Investment,
    pub retirement: Investment,
}

#[derive(Deserialize)]
pub struct Investment {
    pub beginning: Dollars,
    pub ending: Dollars,
    pub deposits: Dollars,
}

pub struct Account {
    pub transactions: Vec<Transaction>,
}

pub enum Kind {
    Withdrawal,
    Deposit,
}

pub struct Category {
    index: usize,
}

impl Category {
    pub fn from_description(config: &Config, description: &str) -> Result<Self> {
        let mut result = None;

        for (i, (cat_name, cat_config)) in config.categories.iter().enumerate() {
            if cat_config.matchers.is_match(description) {
                if let Some(j) = result {
                    bail!(
                        "multiple categories matched transaction '{}': {} and {}",
                        description,
                        config.categories.get_index(j).unwrap().0,
                        cat_name,
                    );
                } else {
                    result = Some(i);
                }
            }
        }

        let index =
            result.ok_or_else(|| anyhow!("no categories matched transaction '{}'", description))?;
        Ok(Self { index })
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

pub struct Transaction {
    pub date: PlainDate,
    pub amount: Dollars,
    #[expect(unused)]
    pub kind: Kind,
    pub description: String,
    pub category: Category,
}
