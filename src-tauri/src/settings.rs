use std::collections::BTreeMap;

use rusqlite::params;
use url::Url;

use crate::{database::Database, error::AppError};

const EDITABLE_KEYS: &[&str] = &[
    "fiverr_gig_url", "normal_lead_limit", "follow_up_delay_days", "second_follow_up_enabled",
    "sender_name", "sender_business_name", "sender_postal_address", "opt_out_line_en",
    "opt_out_line_fr", "backup_retention_count",
];

impl Database {
    pub fn settings(&self) -> Result<BTreeMap<String, String>, AppError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT key, value FROM settings ORDER BY key")?;
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<BTreeMap<_, _>, _>>().map_err(Into::into)
    }

    pub fn update_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        if !EDITABLE_KEYS.contains(&key) { return Err(AppError::InvalidLead("unknown setting".into())); }
        let value = value.trim();
        if value.len() > 2_000 { return Err(AppError::InvalidLead("setting is too long".into())); }
        match key {
            "fiverr_gig_url" => {
                let url = Url::parse(value).map_err(|_| AppError::InvalidLead("Fiverr Gig URL is invalid".into()))?;
                if url.scheme() != "https" || url.host_str().map(|host| !host.ends_with("fiverr.com")).unwrap_or(true) {
                    return Err(AppError::InvalidLead("use an HTTPS fiverr.com Gig URL".into()));
                }
            }
            "normal_lead_limit" => validate_number(value, 1, 10)?,
            "follow_up_delay_days" => validate_number(value, 1, 30)?,
            "backup_retention_count" => validate_number(value, 1, 30)?,
            "second_follow_up_enabled" if !matches!(value, "true" | "false") => return Err(AppError::InvalidLead("invalid yes/no value".into())),
            _ => {}
        }
        let connection = self.connection()?;
        connection.execute("UPDATE settings SET value = ?2, updated_at = CURRENT_TIMESTAMP WHERE key = ?1", params![key, value])?;
        Ok(())
    }
}

fn validate_number(value: &str, minimum: i64, maximum: i64) -> Result<(), AppError> {
    let parsed = value.parse::<i64>().map_err(|_| AppError::InvalidLead("setting must be a number".into()))?;
    if !(minimum..=maximum).contains(&parsed) { return Err(AppError::InvalidLead(format!("setting must be between {minimum} and {maximum}"))); }
    Ok(())
}
