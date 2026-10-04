mod config;
mod data;
mod dollars;
mod render;
mod report;

use core::{
    ops::{Add, AddAssign},
    str::FromStr as _,
};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use indexmap::IndexMap;
use temporal_rs::{
    Duration, PlainDate,
    options::{DifferenceSettings, RoundingMode, Unit},
};

use crate::{config::*, data::*, dollars::*, report::*};

const SECONDS_PER_YEAR: f64 = 60.0 * 60.0 * 24.0 * 365.25;

/// Finance report generator
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The name of the data directory adjacent to the configuration file
    data_name: String,

    /// The path to the configuration file
    #[arg(short, long, default_value = "data/config.json")]
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
    /// JSON formatting
    Json,
    /// HTML formatting
    Html,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config = Config::read(&args.config)?;
    let data_path = args
        .config
        .parent()
        .context("config path has no parent")?
        .join(args.data_name);
    let data = Data::read(&config, &data_path)?;

    let report = generate_report(&config, &data)?;

    let output = if let Some(path) = args.output {
        Box::new(File::create(path)?) as Box<dyn Write>
    } else {
        Box::new(std::io::stdout()) as Box<dyn Write>
    };
    let mut writer = BufWriter::new(output);

    match args.format {
        OutputFormat::Json => render::json_report(&mut writer, &report)?,
        OutputFormat::Html => render::html_report(&mut writer, &report)?,
    }

    Ok(())
}

fn generate_report(config: &Config, data: &Data) -> Result<Report> {
    let totals = calculate_totals(config, data)?;

    let income = Group::calculate(config, &config.income, &totals);
    let shared_expenses = Group::calculate(config, &config.shared_expenses, &totals);
    let individual_expenses = Group::calculate(config, &config.individual_expenses, &totals);

    let mut transactions_by_category = vec![Vec::new(); config.categories.len()];
    let end_date = data.meta.end_date.clone();
    let start_date = end_date.subtract(&Duration::from_str("P1Y")?, None)?;
    let prev_end_date = end_date.subtract(&Duration::from_str("P1M")?, None)?;
    let prev_start_date = prev_end_date.subtract(&Duration::from_str("P1Y")?, None)?;

    for account in data.accounts.iter() {
        for transaction in account.transactions.iter() {
            let cmp_start = transaction.date.compare_iso(&prev_end_date);
            let cmp_end = transaction.date.compare_iso(&end_date);
            if cmp_start.is_gt() && cmp_end.is_le() {
                transactions_by_category[transaction.category.index()].push(transaction);
            }
        }
    }

    let spending_total = shared_expenses.total + individual_expenses.total;
    let net_savings = income.total + spending_total;

    let total_investments =
        data.meta.investments.brokerage.ending + data.meta.investments.retirement.ending;

    let mut retirement = InvestmentsReport {
        retirement_projections: Vec::new(),
        accounts: report_investments(&data.meta.investments),
    };
    for rate_percent in 4..=10 {
        let rate = 0.01 * rate_percent as f64;
        let investments_needed =
            Dollars::from_f64(spending_total.yearly_after_last_month.to_f64().abs() / rate);

        let mut difference_settings = DifferenceSettings::default();
        difference_settings.largest_unit = Some(Unit::Year);
        difference_settings.smallest_unit = Some(Unit::Day);
        difference_settings.rounding_mode = Some(RoundingMode::Ceil);

        let years_all = years_until(
            total_investments.to_f64(),
            net_savings.yearly_after_last_month.to_f64(),
            1.0 + rate,
            investments_needed.to_f64(),
        );
        let secs_all = (years_all * SECONDS_PER_YEAR).floor() as i64;
        let retire_date_all = end_date
            .add(
                &Duration::new(0, 0, 0, 0, 0, 0, secs_all, 0, 0, 0).unwrap(),
                None,
            )
            .unwrap();
        let retire_time_until_all = end_date
            .until(&retire_date_all, difference_settings)
            .unwrap();

        let years_bkg_only = years_until(
            data.meta.investments.brokerage.ending.to_f64(),
            net_savings.yearly_after_last_month.to_f64(),
            1.0 + rate,
            investments_needed.to_f64(),
        );
        let secs_bkg_only = (years_bkg_only * SECONDS_PER_YEAR).floor() as i64;
        let retire_date_bkg_only = end_date
            .add(
                &Duration::new(0, 0, 0, 0, 0, 0, secs_bkg_only, 0, 0, 0).unwrap(),
                None,
            )
            .unwrap();
        let retire_time_until_bkg_only = end_date
            .until(&retire_date_bkg_only, difference_settings)
            .unwrap();

        retirement
            .retirement_projections
            .push(RetirementProjection {
                percent_apy: rate_percent,
                total_needed: investments_needed,
                brokerage_only: RetirementDate {
                    date: retire_date_bkg_only,
                    time_until: retire_time_until_bkg_only,
                },
                all_investments: RetirementDate {
                    date: retire_date_all,
                    time_until: retire_time_until_all,
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

    let yearly = report_yearly(
        start_date,
        end_date,
        income.total.yearly_after_last_month,
        shared_expenses.total.yearly_after_last_month,
        individual_expenses.total.yearly_after_last_month,
    );
    let prev_yearly = report_yearly(
        prev_start_date,
        prev_end_date,
        income.total.yearly_before_last_month,
        shared_expenses.total.yearly_before_last_month,
        individual_expenses.total.yearly_before_last_month,
    );
    let mut difference = DifferenceSettings::default();
    difference.largest_unit = Some(Unit::Day);
    let dev_yearly_from_prev_yearly = YearlyDeviation {
        period_start: yearly
            .period_start
            .since(&prev_yearly.period_start, difference)
            .unwrap(),
        period_end: yearly
            .period_end
            .since(&prev_yearly.period_end, difference)
            .unwrap(),
        total_income: yearly.total_income - prev_yearly.total_income,
        total_spending: yearly.total_spending - prev_yearly.total_spending,
        net_savings: yearly.net_savings - prev_yearly.net_savings,
        net_savings_per_month: yearly.net_savings_per_month - prev_yearly.net_savings_per_month,
        spending_income_ratio: yearly.spending_income_ratio - prev_yearly.spending_income_ratio,
        individual_shared_ratio: yearly.individual_shared_ratio
            - prev_yearly.individual_shared_ratio,
    };

    Ok(Report {
        yearly,
        prev_yearly,
        dev_yearly_from_prev_yearly,

        income: income.report(config),
        shared_expenses: shared_expenses.report(config),
        individual_expenses: individual_expenses.report(config),

        investments: retirement,
        categorized_transactions,
    })
}

fn report_yearly(
    period_start: PlainDate,
    period_end: PlainDate,
    income: Dollars,
    shared_expenses: Dollars,
    individual_expenses: Dollars,
) -> YearlyReport {
    let total_spending = shared_expenses + individual_expenses;
    let net_savings = income + total_spending;

    YearlyReport {
        period_start,
        period_end,
        total_income: income,
        total_spending,
        net_savings,
        net_savings_per_month: Dollars::from_f64(net_savings.to_f64() / 12.0),
        spending_income_ratio: total_spending.to_f64().abs() / income.to_f64() * 100.0,
        individual_shared_ratio: individual_expenses.to_f64() / shared_expenses.to_f64() * 100.0,
    }
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

fn report_investments(investments: &Investments) -> InvestmentAccountsReport {
    let total = Investment {
        beginning: investments.brokerage.beginning + investments.retirement.beginning,
        ending: investments.brokerage.ending + investments.retirement.ending,
        deposits: investments.brokerage.deposits + investments.retirement.deposits,
    };

    InvestmentAccountsReport {
        brokerage: report_investment(&investments.brokerage),
        retirement: report_investment(&investments.retirement),
        total: report_investment(&total),
    }
}

fn report_investment(investment: &Investment) -> InvestmentAccountReport {
    let return_total = investment.ending - investment.beginning - investment.deposits;
    let return_rate = return_total.to_f64() / investment.beginning.to_f64() * 100.0;

    InvestmentAccountReport {
        beginning: investment.beginning,
        ending: investment.ending,
        deposits: investment.deposits,
        return_total,
        return_rate,
    }
}

fn total_categories(
    config: &Config,
    data: &Data,
    start_date: &PlainDate,
    end_date: &PlainDate,
) -> Vec<Dollars> {
    let mut category_totals = vec![Dollars::ZERO; config.categories.len()];

    for account in data.accounts.iter() {
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
    // Total spending based on a month of data from the previous month last year
    pub prev_last_month: Dollars,
    // Total spending based on a year of data ending before the previous month
    pub yearly_before_last_month: Dollars,
    // Total spending based on a month of data from the previous month
    pub last_month: Dollars,
    // Total spending based on a year of data ending after the previous month
    pub yearly_after_last_month: Dollars,
}

struct Subtotals {
    last: Dollars,
    p_last: Dollars,
    avg: Dollars,
    p_avg: Dollars,
    dev_last_from_p_last: Dollars,
    dev_last_from_p_avg: Dollars,
    dev_avg_from_p_avg: Dollars,
}

impl Add for Total {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            prev_last_month: self.prev_last_month + rhs.prev_last_month,
            yearly_before_last_month: self.yearly_before_last_month + rhs.yearly_before_last_month,
            last_month: self.last_month + rhs.last_month,
            yearly_after_last_month: self.yearly_after_last_month + rhs.yearly_after_last_month,
        }
    }
}

impl AddAssign for Total {
    fn add_assign(&mut self, rhs: Self) {
        self.prev_last_month += rhs.prev_last_month;
        self.yearly_before_last_month += rhs.yearly_before_last_month;
        self.last_month += rhs.last_month;
        self.yearly_after_last_month += rhs.yearly_after_last_month;
    }
}

fn calculate_totals(config: &Config, data: &Data) -> Result<Vec<Total>> {
    let end_date = data.meta.end_date.clone();
    let start_date = end_date.subtract(&Duration::from_str("P1Y")?, None)?;
    let prev_end_date = end_date.subtract(&Duration::from_str("P1M")?, None)?;
    let prev_start_date = prev_end_date.subtract(&Duration::from_str("P1Y")?, None)?;

    let prev_last_months = total_categories(config, data, &prev_start_date, &start_date);
    let yearly_before_last_months =
        total_categories(config, data, &prev_start_date, &prev_end_date);
    let last_months = total_categories(config, data, &prev_end_date, &end_date);
    let yearly_after_last_months = total_categories(config, data, &start_date, &end_date);

    let mut totals = Vec::new();
    for i in 0..config.categories.len() {
        totals.push(Total {
            prev_last_month: prev_last_months[i],
            yearly_before_last_month: yearly_before_last_months[i],
            last_month: last_months[i],
            yearly_after_last_month: yearly_after_last_months[i],
        });
    }

    Ok(totals)
}

impl Total {
    fn subtotals(&self) -> Subtotals {
        let last = self.last_month;
        let p_last = self.prev_last_month;
        let avg = Dollars::from_f64(self.yearly_after_last_month.to_f64() / 12.0);
        let p_avg = Dollars::from_f64(self.yearly_before_last_month.to_f64() / 12.0);

        Subtotals {
            last,
            p_last,
            avg,
            p_avg,
            dev_last_from_p_last: last - p_last,
            dev_last_from_p_avg: last - p_avg,
            dev_avg_from_p_avg: avg - p_avg,
        }
    }

    fn report(&self, all: &Self) -> GroupReportRow {
        let self_subtotals = self.subtotals();
        let all_subtotals = all.subtotals();

        GroupReportRow {
            last_subtotal: self_subtotals.last,
            last_percent: percent(self_subtotals.last, all_subtotals.last),
            p_last_subtotal: self_subtotals.p_last,
            p_last_percent: percent(self_subtotals.p_last, all_subtotals.p_last),
            avg_subtotal: self_subtotals.avg,
            avg_percent: percent(self_subtotals.avg, all_subtotals.avg),
            p_avg_subtotal: self_subtotals.p_avg,
            p_avg_percent: percent(self_subtotals.p_avg, all_subtotals.p_avg),
            dev_last_from_p_last_subtotal: self_subtotals.dev_last_from_p_last,
            dev_last_from_p_last_percent: percent(
                self_subtotals.dev_last_from_p_last,
                self_subtotals.p_last,
            ),
            dev_last_from_p_avg_subtotal: self_subtotals.dev_last_from_p_avg,
            dev_last_from_p_avg_percent: percent(
                self_subtotals.dev_last_from_p_avg,
                all_subtotals.p_avg,
            ),
            dev_avg_from_p_avg_subtotal: self_subtotals.dev_avg_from_p_avg,
            dev_avg_from_p_avg_percent: percent(
                self_subtotals.dev_avg_from_p_avg,
                all_subtotals.p_avg,
            ),
        }
    }
}

fn percent(part: Dollars, whole: Dollars) -> f64 {
    let part = part.to_f64();
    let whole = whole.to_f64();
    let result = part / whole * 100.0;

    if (part == 0.0 && whole == 0.0) || result == 0.0 {
        0.0
    } else if result.is_infinite() {
        100.0 * result.signum()
    } else {
        result
    }
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

        Group {
            total,
            per_category,
        }
    }

    fn report(&self, config: &Config) -> GroupReport {
        let mut categories = IndexMap::new();
        for (c, total) in self.per_category.iter() {
            categories.insert(
                config.categories.get_index(*c).unwrap().0.clone(),
                total.report(&self.total),
            );
        }
        categories.sort_by_key(|_, row| -row.last_subtotal.to_cents().abs());

        GroupReport {
            categories,
            total: self.total.report(&self.total),
        }
    }
}
