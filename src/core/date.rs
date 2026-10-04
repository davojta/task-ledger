use jiff::civil::Date;

use crate::core::error::{LedgerError, Result};

pub const TODAY_ENV: &str = "LEDGER_TODAY";

/// Today's date as `YYYY-MM-DD`. `LEDGER_TODAY` overrides it (used by tests).
pub fn today() -> Result<String> {
    match std::env::var(TODAY_ENV) {
        Ok(value) if is_valid_date(&value) => Ok(value),
        Ok(value) => Err(LedgerError::Invalid(format!(
            "{TODAY_ENV}={value} is not a YYYY-MM-DD date"
        ))),
        Err(_) => Ok(jiff::Zoned::now().date().to_string()),
    }
}

/// True when `s` is a valid `YYYY-MM-DD` calendar date.
pub fn is_valid_date(s: &str) -> bool {
    s.len() == 10 && s.parse::<Date>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_dates() {
        assert!(is_valid_date("2026-10-04"));
        assert!(is_valid_date("2024-02-29"));
    }

    #[test]
    fn invalid_dates() {
        for s in ["2026-02-30", "2026-1-4", "20261004", "", "2026-10-04T00:00"] {
            assert!(!is_valid_date(s), "{s}");
        }
    }
}
