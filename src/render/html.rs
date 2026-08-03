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
    report: &'a GroupReport,
}

pub fn html_report(writer: &mut dyn io::Write, report: &Report) -> io::Result<()> {
    ReportTemplate { report }.write_into(writer)
}
