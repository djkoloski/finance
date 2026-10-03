use std::io;

use askama::Template;

use crate::report::{GroupReport, Report};

#[derive(Template)]
#[template(path = "report.html")]
struct ReportTemplate<'a> {
    report: &'a Report,
}

#[derive(Template)]
#[template(path = "group.html")]
struct GroupTemplate<'a> {
    group: &'a GroupReport,
}

pub fn html_report(writer: &mut dyn io::Write, report: &Report) -> io::Result<()> {
    ReportTemplate { report }.write_into(writer)
}

mod filters {
    use core::fmt::Write as _;
    use temporal_rs::{Duration, PlainDate, Sign};

    #[askama::filter_fn]
    pub fn date(date: &PlainDate, _: &dyn askama::Values) -> askama::Result<String> {
        // const MONTH_SHORT: [&str; 12] = [
        //     "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
        // ];
        const MONTH_LONG: [&str; 12] = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        Ok(format!(
            "{} {}, {}",
            MONTH_LONG[date.month() as usize - 1],
            date.day(),
            date.year()
        ))
    }

    #[askama::filter_fn]
    pub fn duration(duration: &Duration, _: &dyn askama::Values) -> askama::Result<String> {
        let sign = duration.sign();
        let duration = duration.abs();

        let mut result = String::new();
        let mut comma = false;

        if sign == Sign::Negative {
            write!(&mut result, "-").unwrap();
        }

        if duration.years() != 0 {
            write!(&mut result, "{}y", duration.years()).unwrap();
            comma = true;
        }

        if duration.months() != 0 {
            if comma {
                write!(&mut result, " ").unwrap();
            }
            write!(&mut result, "{}m", duration.months()).unwrap();
            comma = true;
        }

        if duration.days() != 0 {
            if comma {
                write!(&mut result, " ").unwrap();
            }
            write!(&mut result, "{}d", duration.days()).unwrap();
        }

        Ok(result)
    }
}
