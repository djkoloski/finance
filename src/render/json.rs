use std::io;

use crate::report::Report;

pub fn json_report(writer: &mut dyn io::Write, report: &Report) -> Result<(), serde_json::Error> {
    serde_json::to_writer(writer, report)
}
