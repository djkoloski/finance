mod config;
mod data;
mod render;
mod report;

use core::{
    fmt,
    ops::{Add, AddAssign},
    str::FromStr as _,
};
use std::{fs::File, io::{BufWriter, Write}, path::{Path, PathBuf}};

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use indexmap::IndexMap;
use temporal_rs::{Duration, PlainDate};

use crate::{config::*, data::*, report::*};

const SECONDS_PER_YEAR: f64 = 60.0 * 60.0 * 24.0 * 365.25;

/// Finance report generator
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The path to the configuration file
    #[arg(default_value = "data/config.json")]
    config: PathBuf,

    /// The output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Html)]
    format: OutputFormat,

    /// The output file
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    /// Text formatting
    Text,
    /// JSON formatting
    Json,
    /// HTML formatting
    Html,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let report = generate_report(&args.config)?;

    let output = if let Some(path) = args.output {
        Box::new(File::create(path)?) as Box<dyn Write>
    } else {
        Box::new(std::io::stdout()) as Box<dyn Write>
    };
    let mut writer = BufWriter::new(output);

    match args.format {
        OutputFormat::Text => render::print_report(&mut writer, &report)?,
        OutputFormat::Json => render::json_report(&mut writer, &report)?,
        OutputFormat::Html => render::html_report(&mut writer, &report)?,
    }

    Ok(())
}

fn generate_report(config_path: &Path) -> Result<Report> {
    let config = Config::read(config_path)?;
    let accounts = load_accounts(config_path, &config)?;

    let totals = calculate_totals(&config, &accounts)?;

    let income = Group::calculate(&config, &config.income, &totals);
    let shared_expenses = Group::calculate(&config, &config.shared_expenses, &totals);
    let individual_expenses = Group::calculate(&config, &config.individual_expenses, &totals);

    let mut transactions_by_category = vec![Vec::new(); config.categories.len()];
    let start_date = config
        .end_date
        .subtract(&Duration::from_str("P1M")?, None)?;
    for account in accounts.iter() {
        for transaction in account.transactions.iter() {
            let cmp_start = transaction.date.compare_iso(&start_date);
            let cmp_end = transaction.date.compare_iso(&config.end_date);
            if cmp_start.is_gt() && cmp_end.is_le() {
                transactions_by_category[transaction.category.index()].push(transaction);
            }
        }
    }
    for category in transactions_by_category.iter_mut() {
        category.sort_unstable_by(|a, b| a.amount.total_cmp(&b.amount));
    }

    let income_total = income.total.yearly_after_last_month;
    let spending_total = shared_expenses.total.yearly_after_last_month
        + individual_expenses.total.yearly_after_last_month;
    let net_savings = income_total + spending_total;

    let total_investments = config.investments.brokerage + config.investments.retirement;

    let mut retirement = RetirementReport {
        projections: Vec::new(),
    };
    for rate_percent in 4..=10 {
        let rate = 0.01 * rate_percent as f64;
        let investments_needed = spending_total.abs() / rate;

        let years_all = years_until(
            total_investments,
            net_savings,
            1.0 + rate,
            investments_needed,
        );
        let secs_all = (years_all * SECONDS_PER_YEAR).floor() as i64;
        let retire_date_all = config
            .end_date
            .add(
                &Duration::new(0, 0, 0, 0, 0, 0, secs_all, 0, 0, 0).unwrap(),
                None,
            )
            .unwrap();

        let years_bkg_only = years_until(
            config.investments.brokerage,
            net_savings,
            1.0 + rate,
            investments_needed,
        );
        let secs_bkg_only = (years_bkg_only * SECONDS_PER_YEAR).floor() as i64;
        let retire_date_bkg_only = config
            .end_date
            .add(
                &Duration::new(0, 0, 0, 0, 0, 0, secs_bkg_only, 0, 0, 0).unwrap(),
                None,
            )
            .unwrap();

        retirement.projections.push(RetirementProjection {
            percent_apy: rate_percent,
            total_needed: investments_needed,
            brokerage_only: RetirementDate {
                date: retire_date_bkg_only,
                years_away: years_bkg_only,
            },
            all_investments: RetirementDate {
                date: retire_date_all,
                years_away: years_all,
            },
        });
    }

    let mut categorized_transactions = Vec::new();
    for (c, ts) in transactions_by_category.iter().enumerate() {
        if !ts.is_empty() {
            let mut transactions = Vec::new();
            for t in ts {
                transactions.push(ReportTransaction {
                    amount: t.amount,
                    date: t.date.clone(),
                    description: t.description.clone(),
                });
            }
            transactions.sort_by(|a, b| a.date.compare_iso(&b.date));
            categorized_transactions.push(CategorizedTransactions {
                name: config.categories.get_index(c).unwrap().0.clone(),
                transactions,
            });
        }
    }

    Ok(Report {
        period_start: start_date,
        period_end: config.end_date.clone(),
        income: income.report(&config),
        shared_expenses: shared_expenses.report(&config),
        individual_expenses: individual_expenses.report(&config),
        overall: OverallReport {
            net_savings,
            net_savings_per_month: net_savings / 12.0,
            spending_income_ratio: spending_total.abs() / income_total * 100.0,
            savings_income_ratio: net_savings / income_total * 100.0,
        },
        retirement,
        categorized_transactions,
    })
}

fn compound_with_contribution(p: f64, c: f64, r: f64, n: f64) -> f64 {
    p * r.powf(n) + c / (1.0 - r) * (1.0 - r.powf(n + 1.0)) - c
}

fn cwc_slope(p: f64, c: f64, r: f64, n: f64) -> f64 {
    p * r.powf(n) * r.ln() - c / (1.0 - r) * r.powf(n + 1.0) * r.ln()
}

fn years_until(p: f64, c: f64, r: f64, total: f64) -> f64 {
    let mut x = 1.0;
    for _ in 0..5 {
        let y = compound_with_contribution(p, c, r, x) - total;
        let yp = cwc_slope(p, c, r, x);
        x -= y / yp;
    }
    x
}

pub struct Dollars(pub f64);

impl fmt::Display for Dollars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cents = (self.0.abs() * 100.0).round() as i64;

        let digits = usize::max(cents.checked_ilog10().unwrap_or(0) as usize + 1, 3);
        let width = if self.0 < 0.0 || f.sign_plus() { 1 } else { 0 } // sign
            + 1 // $
            + digits // digits
            + digits.saturating_sub(3) / 3 // separators
            + 1; // .
        if f.align().is_some_and(|a| a == fmt::Alignment::Right)
            && let Some(w) = f.width()
            && width < w
        {
            write!(f, "{:1$}", "", w - width)?;
        }

        if self.0 < 0.0 {
            write!(f, "-")?;
        } else if f.sign_plus() {
            write!(f, "+")?;
        }
        write!(f, "$")?;

        if cents >= 100 {
            let mut dollars = cents / 100;
            let mut digits = dollars.ilog10() + 1;
            let mut is_first_group = true;
            while digits > 0 {
                let mut group_size = digits % 3;
                if group_size == 0 {
                    group_size = 3;
                }
                let base = 10_i64.pow(digits - group_size);
                let segment = dollars / base;
                if is_first_group {
                    write!(f, "{segment}")?;
                    is_first_group = false;
                } else {
                    write!(f, ",{segment:03}")?;
                }
                dollars -= segment * base;
                digits -= group_size;
            }
        } else {
            write!(f, "0")?;
        }

        write!(f, ".{:02}", cents % 100)?;

        if f.align().is_none_or(|a| a == fmt::Alignment::Left)
            && let Some(w) = f.width()
            && width < w
        {
            write!(f, "{:1$}", "", w - width)?;
        }

        Ok(())
    }
}

fn load_accounts(config_path: &Path, config: &Config) -> Result<Vec<Account>> {
    let mut accounts = Vec::new();

    for account_config in config.accounts.iter() {
        let mut transactions = Vec::new();

        let base = config_path.parent().context("config path has no parent")?;
        let path = base.join(&account_config.file);
        let mut reader = csv::Reader::from_reader(File::open(&path)?);
        for result in reader.records() {
            let record = result?;

            let transaction = Transaction {
                date: PlainDate::from_str(&record[0])?,
                amount: record[2].parse()?,
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

    Ok(accounts)
}

fn total_categories(
    config: &Config,
    accounts: &[Account],
    start_date: &PlainDate,
    end_date: &PlainDate,
) -> Vec<f64> {
    let mut category_totals = vec![0.0; config.categories.len()];

    for account in accounts.iter() {
        for transaction in account.transactions.iter() {
            let cmp_start = start_date.compare_iso(&transaction.date);
            let cmp_end = end_date.compare_iso(&transaction.date);
            if cmp_start.is_lt() && cmp_end.is_ge() {
                category_totals[transaction.category.index()] += transaction.amount;
            }
        }
    }

    category_totals
}

#[derive(Clone, Copy, Default)]
struct Total {
    // Total spending based on a year of data ending before the previous month
    pub yearly_before_last_month: f64,
    // Total spending based on a month of data from the previous month
    pub last_month: f64,
    // Total spending based on a year of data ending after the previous month
    pub yearly_after_last_month: f64,
}

impl Add for Total {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            yearly_before_last_month: self.yearly_before_last_month + rhs.yearly_before_last_month,
            last_month: self.last_month + rhs.last_month,
            yearly_after_last_month: self.yearly_after_last_month + rhs.yearly_after_last_month,
        }
    }
}

impl AddAssign for Total {
    fn add_assign(&mut self, rhs: Self) {
        self.yearly_before_last_month += rhs.yearly_before_last_month;
        self.last_month += rhs.last_month;
        self.yearly_after_last_month += rhs.yearly_after_last_month;
    }
}

fn calculate_totals(config: &Config, accounts: &[Account]) -> Result<Vec<Total>> {
    let end_date = config.end_date.clone();
    let start_date = end_date.subtract(&Duration::from_str("P1Y")?, None)?;
    let prev_end_date = end_date.subtract(&Duration::from_str("P1M")?, None)?;
    let prev_start_date = prev_end_date.subtract(&Duration::from_str("P1Y")?, None)?;

    let yearly_before_last_months =
        total_categories(config, accounts, &prev_start_date, &prev_end_date);
    let last_months = total_categories(config, accounts, &prev_end_date, &end_date);
    let yearly_after_last_months = total_categories(config, accounts, &start_date, &end_date);

    let mut totals = Vec::new();
    for i in 0..config.categories.len() {
        totals.push(Total {
            yearly_before_last_month: yearly_before_last_months[i],
            last_month: last_months[i],
            yearly_after_last_month: yearly_after_last_months[i],
        });
    }

    Ok(totals)
}

struct Group {
    pub total: Total,
    pub per_category: Vec<(usize, Total)>,
}

impl Group {
    fn calculate(config: &Config, group_config: &GroupConfig, totals: &[Total]) -> Group {
        let mut total = Total::default();
        let mut per_category = Vec::new();

        for include in group_config.include.iter() {
            let i = config.categories.get_index_of(include).unwrap();
            total += totals[i];
            per_category.push((i, totals[i]));
        }

        let total_sign = total.yearly_after_last_month.signum();
        per_category.sort_unstable_by(|lhs, rhs| {
            let l = lhs.1.yearly_after_last_month * total_sign;
            let r = rhs.1.yearly_after_last_month * total_sign;
            r.total_cmp(&l)
        });

        Group {
            total,
            per_category,
        }
    }

    fn report(&self, config: &Config) -> GroupReport {
        GroupReport {
            last_month: self.report_last_month(config),
            average_month: self.report_average_month(config),
        }
    }

    fn report_last_month(&self, config: &Config) -> LastMonthReport {
        let mut categories = IndexMap::new();
        for (c, total) in self.per_category.iter() {
            categories.insert(
                config.categories.get_index(*c).unwrap().0.clone(),
                LastMonthReportRow {
                    subtotal: total.last_month,
                    percent: total.last_month / self.total.last_month * 100.0,
                    expected: total.yearly_before_last_month / 12.0,
                    deviation: (total.last_month * 12.0 / total.yearly_before_last_month - 1.0)
                        * 100.0,
                },
            );
        }
        LastMonthReport {
            categories,
            total: LastMonthReportRow {
                subtotal: self.total.last_month,
                percent: 100.0,
                expected: self.total.yearly_before_last_month / 12.0,
                deviation: (self.total.last_month * 12.0 / self.total.yearly_before_last_month
                    - 1.0)
                    * 100.0,
            },
        }
    }

    fn report_average_month(&self, config: &Config) -> AverageMonthReport {
        let mut categories = IndexMap::new();
        for (c, total) in self.per_category.iter() {
            categories.insert(
                config.categories.get_index(*c).unwrap().0.clone(),
                AverageMonthReportRow {
                    subtotal: total.yearly_after_last_month / 12.0,
                    percent: total.yearly_after_last_month / self.total.yearly_after_last_month
                        * 100.0,
                    previous: total.yearly_before_last_month / 12.0,
                    change: (total.yearly_after_last_month / total.yearly_before_last_month - 1.0)
                        * 100.0,
                },
            );
        }
        AverageMonthReport {
            categories,
            total: AverageMonthReportRow {
                subtotal: self.total.yearly_after_last_month / 12.0,
                percent: 100.0,
                previous: self.total.yearly_before_last_month / 12.0,
                change: (self.total.yearly_after_last_month / self.total.yearly_before_last_month
                    - 1.0)
                    * 100.0,
            },
            yearly_subtotal: self.total.yearly_after_last_month,
            prev_yearly_subtotal: self.total.yearly_before_last_month,
        }
    }
}
