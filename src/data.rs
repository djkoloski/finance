use anyhow::{Result, anyhow, bail};
use temporal_rs::PlainDate;

use crate::{config::Config, dollars::Dollars};

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
