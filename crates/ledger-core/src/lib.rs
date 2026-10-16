use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("{0}")]
    Invalid(String),
}
pub type Result<T> = std::result::Result<T, ValidationError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}
impl AccountKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asset => "asset",
            Self::Liability => "liability",
            Self::Equity => "equity",
            Self::Income => "income",
            Self::Expense => "expense",
        }
    }
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "asset" => Ok(Self::Asset),
            "liability" => Ok(Self::Liability),
            "equity" => Ok(Self::Equity),
            "income" => Ok(Self::Income),
            "expense" => Ok(Self::Expense),
            _ => Err(ValidationError::Invalid("Unknown account type.".into())),
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Debit,
    Credit,
}
impl Side {
    pub fn as_str(self) -> &'static str {
        if self == Self::Debit {
            "debit"
        } else {
            "credit"
        }
    }
    pub fn opposite(self) -> Self {
        if self == Self::Debit {
            Self::Credit
        } else {
            Self::Debit
        }
    }
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "debit" => Ok(Self::Debit),
            "credit" => Ok(Self::Credit),
            _ => Err(ValidationError::Invalid("Unknown posting side.".into())),
        }
    }
}

/// All supported first-milestone currencies have two minor-unit decimal places.
pub fn validate_currency(currency: &str) -> Result<()> {
    if ["INR", "USD", "EUR", "GBP", "CAD", "AUD"].contains(&currency) {
        Ok(())
    } else {
        Err(ValidationError::Invalid(
            "Choose one of the supported two-decimal currencies.".into(),
        ))
    }
}
pub fn validate_date(date: &str) -> Result<()> {
    if date.len() != 10
        || !date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
        || NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err()
    {
        return Err(ValidationError::Invalid(
            "Enter a valid date in YYYY-MM-DD format.".into(),
        ));
    }
    Ok(())
}
pub fn clean_text(value: &str, label: &str, max: usize, required: bool) -> Result<String> {
    let text = value.trim();
    if (required && text.is_empty())
        || text.chars().count() > max
        || text.chars().any(|c| c.is_control())
    {
        return Err(ValidationError::Invalid(format!(
            "{label} must {}contain at most {max} characters and no control characters.",
            if required { "be provided and " } else { "" }
        )));
    }
    Ok(text.to_owned())
}
pub fn validate_account(code: &str, name: &str) -> Result<(String, String)> {
    let code = clean_text(code, "Account code", 24, true)?;
    if !code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err(ValidationError::Invalid(
            "Account codes can contain letters, numbers, and hyphens.".into(),
        ));
    }
    Ok((code, clean_text(name, "Account name", 120, true)?))
}

/// Parse decimal text without ever converting through a binary floating-point value.
pub fn parse_amount(text: &str) -> Result<i64> {
    let invalid = || {
        ValidationError::Invalid("Enter a positive amount with at most two decimal places.".into())
    };
    let text = text.trim();
    let parts: Vec<&str> = text.split('.').collect();
    if parts.len() > 2 || text.is_empty() {
        return Err(invalid());
    }
    let whole = parts[0];
    let fraction = parts.get(1).copied().unwrap_or("");
    if (whole.is_empty() && fraction.is_empty())
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 2
    {
        return Err(invalid());
    }
    let integer = if whole.is_empty() {
        0i64
    } else {
        whole.parse::<i64>().map_err(|_| invalid())?
    };
    let minor = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().map_err(|_| invalid())? * 10,
        _ => fraction.parse::<i64>().map_err(|_| invalid())?,
    };
    let amount = integer
        .checked_mul(100)
        .and_then(|x| x.checked_add(minor))
        .ok_or_else(invalid)?;
    if amount <= 0 {
        return Err(invalid());
    }
    Ok(amount)
}
pub fn decimal_amount(minor: i64) -> String {
    format!("{}.{:02}", minor / 100, minor % 100)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PostingInput {
    pub account_id: i64,
    pub side: Side,
    pub amount: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalInput {
    pub date: String,
    pub reference: String,
    pub description: String,
    pub request_id: String,
    pub lines: Vec<PostingInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Posting {
    pub account_id: i64,
    pub side: Side,
    pub amount_minor: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedJournal {
    pub date: String,
    pub reference: String,
    pub description: String,
    pub request_id: String,
    pub lines: Vec<Posting>,
    pub total_minor: i64,
}
pub fn validate_journal(input: JournalInput) -> Result<ValidatedJournal> {
    validate_date(&input.date)?;
    let request_id = clean_text(&input.request_id, "Request identifier", 128, true)?;
    if !(2..=500).contains(&input.lines.len()) {
        return Err(ValidationError::Invalid(
            "A journal needs between 2 and 500 posting lines.".into(),
        ));
    }
    let mut debit = 0i128;
    let mut credit = 0i128;
    let mut lines = Vec::new();
    for line in input.lines {
        if line.account_id <= 0 {
            return Err(ValidationError::Invalid(
                "Select an account for every posting line.".into(),
            ));
        }
        let amount_minor = parse_amount(&line.amount)?;
        if line.side == Side::Debit {
            debit += i128::from(amount_minor)
        } else {
            credit += i128::from(amount_minor)
        };
        lines.push(Posting {
            account_id: line.account_id,
            side: line.side,
            amount_minor,
        });
    }
    if debit != credit {
        return Err(ValidationError::Invalid(
            "Debits and credits must balance before posting.".into(),
        ));
    }
    let total_minor = i64::try_from(debit).map_err(|_| {
        ValidationError::Invalid("The journal total exceeds the supported amount range.".into())
    })?;
    Ok(ValidatedJournal {
        date: input.date,
        reference: clean_text(&input.reference, "Reference", 64, false)?,
        description: clean_text(&input.description, "Description", 1000, true)?,
        request_id,
        lines,
        total_minor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimal_parser_is_exact_and_rejects_ambiguous_input() {
        for (s, n) in [
            ("0.01", 1),
            ("12.3", 1230),
            (".50", 50),
            ("90071992547409.91", 9007199254740991),
            ("92233720368547758.07", i64::MAX),
        ] {
            assert_eq!(parse_amount(s).unwrap(), n);
        }
        for s in [
            "0",
            "-1",
            "1e3",
            "1,000",
            "NaN",
            "0.001",
            "92233720368547758.08",
            "1.2.3",
            ".",
        ] {
            assert!(parse_amount(s).is_err(), "{s}");
        }
    }
    #[test]
    fn dates_are_real_calendar_dates() {
        assert!(validate_date("2024-02-29").is_ok());
        assert!(validate_date("2026-02-29").is_err());
        assert!(validate_date("2026-13-01").is_err());
    }
    #[test]
    fn balanced_totals_are_validated_with_wide_accumulation() {
        let make = |n| JournalInput {
            date: "2026-10-14".into(),
            reference: "".into(),
            description: "Opening capital".into(),
            request_id: "example".into(),
            lines: vec![
                PostingInput {
                    account_id: 1,
                    side: Side::Debit,
                    amount: "100.01".into(),
                },
                PostingInput {
                    account_id: 2,
                    side: Side::Credit,
                    amount: n,
                },
            ],
        };
        assert_eq!(
            validate_journal(make("100.01".into())).unwrap().total_minor,
            10001
        );
        assert!(validate_journal(make("100.00".into())).is_err());
        for cents in 1..10000i64 {
            let amount = decimal_amount(cents);
            assert_eq!(parse_amount(&amount).unwrap(), cents);
        }
    }
}
