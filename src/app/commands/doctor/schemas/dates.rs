use crate::app::commands::doctor::diagnostics::Diagnostics;
use crate::app::commands::doctor::yaml::get_string;
use chrono::NaiveDate;
use serde_yaml::Mapping;
use std::path::Path;

pub fn ensure_date(map: &Mapping, path: &Path, key: &str, diagnostics: &mut Diagnostics) {
    let value = get_string(map, key).unwrap_or_default();
    if NaiveDate::parse_from_str(&value, "%Y-%m-%d").is_err() {
        diagnostics.push_error(path.display().to_string(), format!("{} must be YYYY-MM-DD", key));
    }
}
