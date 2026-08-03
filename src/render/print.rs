use std::io;

use crate::{
    Dollars,
    report::{GroupReport, Report},
};

pub fn print_report(writer: &mut dyn io::Write, report: &Report) -> io::Result<()> {
    writeln!(
        writer,
        "report for period from {} to {}:",
        report.period_start, report.period_end,
    )?;
    writeln!(writer)?;

    writeln!(writer, "income:")?;
    print_group_report(writer, &report.income)?;

    writeln!(writer, "shared expenses:")?;
    print_group_report(writer, &report.shared_expenses)?;

    writeln!(writer, "individual expenses:")?;
    print_group_report(writer, &report.individual_expenses)?;

    writeln!(writer, "overall:")?;
    writeln!(
        writer,
        "        net savings: {:>12}",
        Dollars(report.overall.net_savings),
    )?;
    writeln!(
        writer,
        "          per month: {:>12}",
        Dollars(report.overall.net_savings_per_month),
    )?;
    writeln!(
        writer,
        "  spending / income: {:>6.2}%",
        report.overall.spending_income_ratio,
    )?;
    writeln!(
        writer,
        "   savings / income: {:>6.2}%",
        report.overall.savings_income_ratio,
    )?;
    writeln!(writer)?;

    writeln!(writer, "{:->80}", "")?;
    writeln!(writer)?;

    writeln!(writer, "retirement calculator:")?;
    writeln!(
        writer,
        "  {:>19}  |  {:^17}  |  {:^17}",
        "", "brokerage only", "all investments",
    )?;
    writeln!(
        writer,
        "  {:^3}  {:^14}  |  {:^10}  {:^5}  |  {:^10}  {:^5}",
        "APY", "total needed", "date", "years", "date", "years",
    )?;

    writeln!(writer, "  {:->63}", "")?;

    for projection in report.retirement.projections.iter() {
        writeln!(
            writer,
            "  {:>2}%  {:>14}  |  {:>10}  {:>5.2}  |  {:>10}  {:>5.2}",
            projection.percent_apy,
            Dollars(projection.total_needed),
            projection.brokerage_only.date,
            projection.brokerage_only.years_away,
            projection.all_investments.date,
            projection.all_investments.years_away,
        )?;
    }
    writeln!(writer)?;

    writeln!(writer, "{:->80}", "")?;
    writeln!(writer)?;

    writeln!(writer, "transactions:")?;
    for categorized_transactions in report.categorized_transactions.iter() {
        writeln!(writer, "  {}:", categorized_transactions.name)?;

        for transaction in categorized_transactions.transactions.iter() {
            write!(
                writer,
                "    {:>12}  {:>10}  ",
                Dollars(transaction.amount),
                transaction.date
            )?;
            if transaction.description.len() > 50 {
                writeln!(writer, "{}...", &transaction.description[..47])?;
            } else {
                writeln!(writer, "{}", transaction.description)?;
            }
        }

        writeln!(writer)?;
    }

    Ok(())
}

fn print_group_report(writer: &mut dyn io::Write, report: &GroupReport) -> io::Result<()> {
    writeln!(writer, "  last month:")?;
    writeln!(
        writer,
        "    {:22}  {:>11}  {:>7}  |  {:>11}  {:>9}",
        "category", "subtotal", "percent", "expected", "deviation",
    )?;
    writeln!(writer, "    {:->71}", "")?;
    for (name, row) in report.last_month.categories.iter() {
        writeln!(
            writer,
            "    {:22}  {:>11}  {:>6.2}%  |  {:>11}  {:>+8.2}%",
            name,
            Dollars(row.subtotal),
            row.percent,
            Dollars(row.expected),
            row.deviation,
        )?;
    }
    writeln!(writer, "    {:->71}", "")?;
    writeln!(
        writer,
        "    {:22}  {:>11}  {:>6.2}%  |  {:>11}  {:>+8.2}%",
        "total",
        Dollars(report.last_month.total.subtotal),
        report.last_month.total.percent,
        Dollars(report.last_month.total.expected),
        report.last_month.total.deviation,
    )?;
    writeln!(writer)?;

    writeln!(writer, "  average month:")?;
    writeln!(
        writer,
        "    {:22}  {:>11}  {:>7}  |  {:>11}  {:>9}",
        "category", "subtotal", "percent", "previous", "change",
    )?;
    writeln!(writer, "    {:->71}", "")?;
    for (name, row) in report.average_month.categories.iter() {
        writeln!(
            writer,
            "    {:22}  {:>11}  {:>6.2}%  |  {:>11}  {:>+8.2}%",
            name,
            Dollars(row.subtotal),
            row.percent,
            Dollars(row.previous),
            row.change,
        )?;
    }
    writeln!(writer, "    {:->71}", "")?;
    writeln!(
        writer,
        "    {:22}  {:>11}  {:>6.2}%  |  {:>11}  {:>+8.2}%",
        "total",
        Dollars(report.average_month.total.subtotal),
        report.average_month.total.percent,
        Dollars(report.average_month.total.previous),
        report.average_month.total.change,
    )?;
    writeln!(
        writer,
        "    total (yearly)          {:>11}           |  {:>11}",
        Dollars(report.average_month.yearly_subtotal),
        Dollars(report.average_month.prev_yearly_subtotal),
    )?;
    writeln!(writer)?;

    Ok(())
}
