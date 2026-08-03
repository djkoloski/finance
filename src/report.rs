use indexmap::IndexMap;
use serde::{Serialize, Serializer};
use temporal_rs::PlainDate;

use crate::dollars::Dollars;

#[derive(Serialize)]
pub struct Report {
    #[serde(serialize_with = "string_date")]
    pub prev_period_start: PlainDate,
    #[serde(serialize_with = "string_date")]
    pub prev_period_end: PlainDate,
    #[serde(serialize_with = "string_date")]
    pub period_start: PlainDate,
    #[serde(serialize_with = "string_date")]
    pub period_end: PlainDate,

    pub yearly: YearlyReport,
    pub prev_yearly: YearlyReport,

    pub income: GroupReport,
    pub shared_expenses: GroupReport,
    pub individual_expenses: GroupReport,

    pub retirement: RetirementReport,
    pub categorized_transactions: Vec<CategorizedTransactions>,
}

#[derive(Serialize)]
pub struct GroupReport {
    pub last_month: LastMonthReport,
    pub average_month: AverageMonthReport,
}

#[derive(Serialize)]
pub struct LastMonthReport {
    pub categories: IndexMap<String, LastMonthReportRow>,
    pub total: LastMonthReportRow,
}

#[derive(Serialize)]
pub struct LastMonthReportRow {
    pub subtotal: Dollars,
    pub percent: f64,
    pub expected: Dollars,
    pub deviation: f64,
}

#[derive(Serialize)]
pub struct AverageMonthReport {
    pub categories: IndexMap<String, AverageMonthReportRow>,
    pub total: AverageMonthReportRow,
    pub yearly_subtotal: Dollars,
    pub prev_yearly_subtotal: Dollars,
}

#[derive(Serialize)]
pub struct AverageMonthReportRow {
    pub subtotal: Dollars,
    pub percent: f64,
    pub previous: Dollars,
    pub change: f64,
}

#[derive(Serialize)]
pub struct YearlyReport {
    pub total_income: Dollars,
    pub total_spending: Dollars,
    pub net_savings: Dollars,
    pub net_savings_per_month: Dollars,
    pub spending_income_ratio: f64,
    pub individual_shared_ratio: f64,
}

#[derive(Serialize)]
pub struct RetirementReport {
    pub projections: Vec<RetirementProjection>,
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
    pub years_away: f64,
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
