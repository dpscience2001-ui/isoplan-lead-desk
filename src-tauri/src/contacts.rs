use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{database::Database, error::AppError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddContactInput {
    pub lead_id: String,
    pub contact_type: String,
    pub value: String,
    pub recipient_role: Option<String>,
    pub source_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactRecord {
    pub id: String,
    pub contact_type: String,
    pub value: String,
    pub recipient_role: Option<String>,
    pub source_url: String,
    pub caution_reason: Option<String>,
}

impl Database {
    pub fn add_contact(&self, input: AddContactInput) -> Result<(), AppError> {
        if !matches!(input.contact_type.as_str(), "role_email" | "named_email" | "contact_form" | "phone") {
            return Err(AppError::InvalidLead("unknown contact type".into()));
        }
        let value = input.value.trim();
        if value.is_empty() || value.len() > 500 { return Err(AppError::InvalidLead("contact value is invalid".into())); }
        if matches!(input.contact_type.as_str(), "role_email" | "named_email") && !valid_email(value) {
            return Err(AppError::InvalidLead("email address is invalid".into()));
        }
        let source = url::Url::parse(input.source_url.trim()).map_err(|_| AppError::InvalidLead("contact source URL is invalid".into()))?;
        if !matches!(source.scheme(), "http" | "https") { return Err(AppError::InvalidLead("contact source must use HTTP or HTTPS".into())); }
        let caution = if input.contact_type == "named_email" { Some("Named personal address: confirm that business outreach is appropriate.") } else { None };
        let connection = self.connection()?;
        let blocked: bool = connection.query_row("SELECT do_not_contact FROM leads WHERE id = ?1", [&input.lead_id], |row| row.get(0))?;
        if blocked { return Err(AppError::DoNotContact); }
        connection.execute(
            "INSERT INTO contacts(id, lead_id, type, value, recipient_role, source_url, caution_reason) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![Uuid::new_v4().to_string(), input.lead_id, input.contact_type, value, input.recipient_role.as_deref().map(str::trim), source.to_string(), caution],
        )?;
        Ok(())
    }

    pub fn contacts(&self, lead_id: &str) -> Result<Vec<ContactRecord>, AppError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT id, type, value, recipient_role, source_url, caution_reason FROM contacts WHERE lead_id = ?1 ORDER BY is_preferred DESC, created_at")?;
        let contacts = statement
            .query_map([lead_id], |row| Ok(ContactRecord { id: row.get(0)?, contact_type: row.get(1)?, value: row.get(2)?, recipient_role: row.get(3)?, source_url: row.get(4)?, caution_reason: row.get(5)? }))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(contacts)
    }
}

fn valid_email(value: &str) -> bool {
    if value.len() > 254 || value.chars().any(char::is_whitespace) { return false; }
    let Some((local, domain)) = value.rsplit_once('@') else { return false; };
    !local.is_empty() && local.len() <= 64 && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}
