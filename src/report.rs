use indexmap::IndexMap;
use serde::{Serialize, Serializer};
use temporal_rs::{Duration, PlainDate};

use crate::dollars::Dollars;

#[derive(Serialize)]
pub struct Report {
    pub yearly: YearlyReport,
    pub prev_yearly: YearlyReport,
    pub dev_yearly_from_prev_yearly: YearlyDeviation,

    pub income: GroupReport,
    pub shared_expenses: GroupReport,
    pub individual_expenses: GroupReport,

    pub investments: InvestmentsReport,
    pub categorized_transactions: Vec<CategorizedTransactions>,
}

#[derive(Serialize)]
pub struct GroupReport {
    pub categories: IndexMap<String, GroupReportRow>,
    pub total: GroupReportRow,
}

#[derive(Serialize)]
pub struct GroupReportRow {
    pub last_subtotal: Dollars,
    pub last_percent: f64,
    pub p_last_subtotal: Dollars,
    pub p_last_percent: f64,
    pub avg_subtotal: Dollars,
    pub avg_percent: f64,
    pub p_avg_subtotal: Dollars,
    pub p_avg_percent: f64,
    pub dev_last_from_p_last_subtotal: Dollars,
    pub dev_last_from_p_last_percent: f64,
    pub dev_last_from_p_avg_subtotal: Dollars,
    pub dev_last_from_p_avg_percent: f64,
    pub dev_avg_from_p_avg_subtotal: Dollars,
    pub dev_avg_from_p_avg_percent: f64,
}

#[derive(Serialize)]
pub struct YearlyReport {
    #[serde(serialize_with = "string_date")]
    pub period_start: PlainDate,
    #[serde(serialize_with = "string_date")]
    pub period_end: PlainDate,

    pub total_income: Dollars,
    pub total_spending: Dollars,
    pub net_savings: Dollars,
    pub net_savings_per_month: Dollars,
    pub spending_income_ratio: f64,
    pub individual_shared_ratio: f64,
}

#[derive(Serialize)]
pub struct YearlyDeviation {
    #[serde(serialize_with = "string_duration")]
    pub period_start: Duration,
    #[serde(serialize_with = "string_duration")]
    pub period_end: Duration,

    pub total_income: Dollars,
    pub total_spending: Dollars,
    pub net_savings: Dollars,
    pub net_savings_per_month: Dollars,
    pub spending_income_ratio: f64,
    pub individual_shared_ratio: f64,
}

#[derive(Serialize)]
pub struct InvestmentsReport {
    pub retirement_projections: Vec<RetirementProjection>,
    pub accounts: InvestmentAccountsReport,
}

#[derive(Serialize)]
pub struct RetirementProjection {
    pub percent_apy: usize,
    pub total_needed: Dollars,
    pub brokerage_only: RetirementDate,
    pub all_investments: RetirementDate,
}

#[derive(Serialize)]
pub struct RetirementDate {
    #[serde(serialize_with = "string_date")]
    pub date: PlainDate,
    #[serde(serialize_with = "string_duration")]
    pub time_until: Duration,
}

#[derive(Serialize)]
pub struct InvestmentAccountsReport {
    pub brokerage: InvestmentAccountReport,
    pub retirement: InvestmentAccountReport,
    pub total: InvestmentAccountReport,
}

#[derive(Serialize)]
pub struct InvestmentAccountReport {
    pub beginning: Dollars,
    pub ending: Dollars,
    pub deposits: Dollars,
    pub return_total: Dollars,
    pub return_rate: f64,
}

#[derive(Serialize)]
pub struct CategorizedTransactions {
    pub name: String,
    pub transactions: Vec<ReportTransaction>,
}

#[derive(Serialize)]
pub struct ReportTransaction {
    pub amount: Dollars,
    #[serde(serialize_with = "string_date")]
    pub date: PlainDate,
    pub description: String,
}

fn string_date<S>(date: &PlainDate, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{date}");
    serializer.serialize_str(&s)
}

fn string_duration<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{duration}");
    serializer.serialize_str(&s)
}
