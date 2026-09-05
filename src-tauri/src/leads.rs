use rusqlite::{params, ErrorCode};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{database::Database, error::AppError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLeadInput {
    pub company_name: String,
    pub official_website: String,
    pub country: String,
    pub language: String,
    pub city_or_service_area: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLeadInput {
    pub id: String,
    pub company_name: String,
    pub official_website: String,
    pub country: String,
    pub language: String,
    pub city_or_service_area: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeadSummary {
    pub id: String,
    pub company_name: String,
    pub official_website: String,
    pub country: String,
    pub language: String,
    pub city_or_service_area: Option<String>,
    pub status: String,
    pub qualification_score: Option<i64>,
    pub confidence_level: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardCounts {
    pub total: i64,
    pub needs_review: i64,
    pub qualified: i64,
    pub drafts_ready: i64,
    pub contacted: i64,
    pub follow_ups_due: i64,
    pub replied: i64,
    pub interested: i64,
    pub converted: i64,
}

impl Database {
    pub fn create_lead(&self, input: CreateLeadInput) -> Result<LeadSummary, AppError> {
        let company_name = input.company_name.trim();
        if company_name.len() < 2 || company_name.len() > 200 {
            return Err(AppError::InvalidLead("company name must be 2–200 characters".into()));
        }
        if !matches!(input.country.as_str(), "GB" | "US" | "FR") {
            return Err(AppError::InvalidLead("country must be GB, US, or FR".into()));
        }
        if !matches!(input.language.as_str(), "en" | "fr") {
            return Err(AppError::InvalidLead("language must be English or French".into()));
        }

        let mut website = Url::parse(input.official_website.trim())
            .map_err(|_| AppError::InvalidLead("official website must be a valid URL".into()))?;
        if !matches!(website.scheme(), "http" | "https") {
            return Err(AppError::InvalidLead("only HTTP and HTTPS websites are supported".into()));
        }
        website.set_fragment(None);
        let domain = website
            .host_str()
            .ok_or_else(|| AppError::InvalidLead("official website must include a domain".into()))?
            .to_ascii_lowercase();
        let normalized_domain = domain.strip_prefix("www.").unwrap_or(&domain).to_owned();
        let official_website = website.to_string().trim_end_matches('/').to_owned();
        let id = Uuid::new_v4().to_string();
        let fingerprint = format!("domain:{normalized_domain}");
        let city = input.city_or_service_area.as_deref().map(str::trim).filter(|v| !v.is_empty());

        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let blocked: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM do_not_contact_entries WHERE normalized_value = ?1)",
            [&normalized_domain],
            |row| row.get(0),
        )?;
        if blocked { return Err(AppError::DoNotContact); }
        let insert = transaction.execute(
            "INSERT INTO leads (
                id, company_name, official_website, normalized_domain, country, language,
                city_or_service_area, duplicate_fingerprint
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, company_name, official_website, normalized_domain, input.country, input.language, city, fingerprint],
        );
        if let Err(error) = insert {
            if matches!(error.sqlite_error_code(), Some(ErrorCode::ConstraintViolation)) {
                return Err(AppError::DuplicateLead);
            }
            return Err(error.into());
        }
        transaction.execute(
            "INSERT INTO timeline_events (id, lead_id, event_type, detail)
             VALUES (?1, ?2, 'lead_created', 'Added manually from an official website')",
            params![Uuid::new_v4().to_string(), id],
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_lead_summary(&id)
    }

    pub fn list_leads(&self) -> Result<Vec<LeadSummary>, AppError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, company_name, official_website, country, language, city_or_service_area,
                    status, qualification_score, confidence_level, created_at
             FROM leads ORDER BY created_at DESC, company_name ASC",
        )?;
        let rows = statement.query_map([], map_lead_summary)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn update_lead(&self, input: UpdateLeadInput) -> Result<LeadSummary, AppError> {
        let company_name = input.company_name.trim();
        if company_name.len() < 2 || company_name.len() > 200 { return Err(AppError::InvalidLead("company name must be 2–200 characters".into())); }
        if !matches!(input.country.as_str(), "GB" | "US" | "FR") { return Err(AppError::InvalidLead("country must be GB, US, or FR".into())); }
        if !matches!(input.language.as_str(), "en" | "fr") { return Err(AppError::InvalidLead("language must be English or French".into())); }
        let mut website = Url::parse(input.official_website.trim()).map_err(|_| AppError::InvalidLead("official website must be a valid URL".into()))?;
        if !matches!(website.scheme(), "http" | "https") { return Err(AppError::InvalidLead("only HTTP and HTTPS websites are supported".into())); }
        website.set_fragment(None);
        let domain = website.host_str().ok_or_else(|| AppError::InvalidLead("official website must include a domain".into()))?.to_ascii_lowercase();
        let normalized_domain = domain.strip_prefix("www.").unwrap_or(&domain).to_owned();
        let official_website = website.to_string().trim_end_matches('/').to_owned();
        let fingerprint = format!("domain:{normalized_domain}");
        let city = input.city_or_service_area.as_deref().map(str::trim).filter(|v| !v.is_empty());
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let is_blocked: bool = transaction.query_row("SELECT do_not_contact FROM leads WHERE id = ?1", [&input.id], |row| row.get(0))?;
        if is_blocked { return Err(AppError::DoNotContact); }
        let dnc_domain: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM do_not_contact_entries WHERE normalized_value = ?1)", [&normalized_domain], |row| row.get(0))?;
        if dnc_domain { return Err(AppError::DoNotContact); }
        let result = transaction.execute(
            "UPDATE leads SET company_name=?2, official_website=?3, normalized_domain=?4, country=?5, language=?6,
                    city_or_service_area=?7, duplicate_fingerprint=?8, updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            params![input.id, company_name, official_website, normalized_domain, input.country, input.language, city, fingerprint],
        );
        if let Err(error) = result {
            if matches!(error.sqlite_error_code(), Some(ErrorCode::ConstraintViolation)) { return Err(AppError::DuplicateLead); }
            return Err(error.into());
        }
        transaction.execute("INSERT INTO timeline_events(id, lead_id, event_type, detail) VALUES(?1, ?2, 'lead_updated', 'Company details edited by user')", params![Uuid::new_v4().to_string(), input.id])?;
        transaction.commit()?;
        drop(connection);
        self.get_lead_summary(&input.id)
    }

    fn get_lead_summary(&self, id: &str) -> Result<LeadSummary, AppError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, company_name, official_website, country, language, city_or_service_area,
                        status, qualification_score, confidence_level, created_at
                 FROM leads WHERE id = ?1",
                [id],
                map_lead_summary,
            )
            .map_err(Into::into)
    }

    pub fn dashboard_counts(&self) -> Result<DashboardCounts, AppError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT
                    COUNT(*),
                    COALESCE(SUM(CASE WHEN status = 'needs_review' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'qualified' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'draft_ready' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'contacted' THEN 1 ELSE 0 END), 0),
                    (SELECT COUNT(*) FROM follow_ups WHERE status IN ('due','snoozed') AND due_at <= CURRENT_TIMESTAMP),
                    COALESCE(SUM(CASE WHEN status = 'replied' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'interested' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'converted' THEN 1 ELSE 0 END), 0)
                 FROM leads",
                [],
                |row| Ok(DashboardCounts {
                    total: row.get(0)?, needs_review: row.get(1)?, qualified: row.get(2)?,
                    drafts_ready: row.get(3)?, contacted: row.get(4)?, follow_ups_due: row.get(5)?,
                    replied: row.get(6)?, interested: row.get(7)?, converted: row.get(8)?,
                }),
            )
            .map_err(Into::into)
    }

    pub fn delete_lead(&self, id: &str, confirmation: &str) -> Result<(), AppError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let (company, status): (String, String) = transaction.query_row("SELECT company_name, status FROM leads WHERE id=?1", [id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        if confirmation != company { return Err(AppError::InvalidLead("type the exact company name to confirm deletion".into())); }
        if !matches!(status.as_str(), "rejected" | "archived" | "do_not_contact") { return Err(AppError::InvalidLead("only rejected, archived, or do-not-contact leads can be deleted".into())); }
        transaction.execute("DELETE FROM leads WHERE id=?1", [id])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_lead_status(&self, id: &str, status: &str, reason: Option<&str>) -> Result<(), AppError> {
        const STATUSES: &[&str] = &[
            "discovered", "researching", "needs_review", "qualified", "rejected", "draft_ready",
            "approved", "gmail_opened", "contacted", "follow_up_due", "replied", "interested",
            "not_interested", "converted", "do_not_contact", "archived",
        ];
        if !STATUSES.contains(&status) {
            return Err(AppError::InvalidLead("unknown pipeline status".into()));
        }
        if status == "do_not_contact" && reason.map(str::trim).filter(|v| !v.is_empty()).is_none() {
            return Err(AppError::InvalidLead("a do-not-contact reason is required".into()));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let (domain, is_blocked): (String, bool) = transaction.query_row(
            "SELECT normalized_domain, do_not_contact FROM leads WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if is_blocked && status != "do_not_contact" { return Err(AppError::DoNotContact); }
        let metadata = format!(r#"{{"status":"{status}"}}"#);
        transaction.execute(
            "UPDATE leads SET status = ?2, do_not_contact = CASE WHEN ?2 = 'do_not_contact' THEN 1 ELSE do_not_contact END,
                    do_not_contact_reason = CASE WHEN ?2 = 'do_not_contact' THEN ?3 ELSE do_not_contact_reason END,
                    updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id, status, reason],
        )?;
        if status == "do_not_contact" {
            transaction.execute(
                "INSERT INTO do_not_contact_entries(id, normalized_value, value_type, reason)
                 VALUES(?1, ?2, 'domain', ?3)
                 ON CONFLICT(normalized_value) DO UPDATE SET reason = excluded.reason",
                params![Uuid::new_v4().to_string(), domain, reason],
            )?;
        }
        transaction.execute(
            "INSERT INTO timeline_events(id, lead_id, event_type, detail, metadata_json)
             VALUES(?1, ?2, 'status_changed', ?3, ?4)",
            params![Uuid::new_v4().to_string(), id, reason, metadata],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn map_lead_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<LeadSummary> {
    Ok(LeadSummary {
        id: row.get(0)?, company_name: row.get(1)?, official_website: row.get(2)?,
        country: row.get(3)?, language: row.get(4)?, city_or_service_area: row.get(5)?,
        status: row.get(6)?, qualification_score: row.get(7)?, confidence_level: row.get(8)?,
        created_at: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::{mpsc, Arc}, time::Duration};

    fn database() -> Arc<Database> {
        let path = std::env::temp_dir().join(format!("isoplan-test-{}.db", Uuid::new_v4()));
        Arc::new(Database::open_path(path).expect("test database"))
    }

    #[test]
    fn create_and_update_return_without_relocking_deadlock() {
        let database = database();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let created = database.create_lead(CreateLeadInput { company_name: "Example Stays".into(), official_website: "https://example.com".into(), country: "GB".into(), language: "en".into(), city_or_service_area: None }).expect("create lead");
            let updated = database.update_lead(UpdateLeadInput { id: created.id, company_name: "Example Stays Ltd".into(), official_website: "https://example.com".into(), country: "GB".into(), language: "en".into(), city_or_service_area: Some("Cornwall".into()) }).expect("update lead");
            sender.send(updated).expect("return result");
        });
        let updated = receiver.recv_timeout(Duration::from_secs(3)).expect("database operation deadlocked");
        assert_eq!(updated.company_name, "Example Stays Ltd");
    }

    #[test]
    fn deletion_requires_terminal_status_and_exact_name() {
        let database = database();
        let lead = database.create_lead(CreateLeadInput { company_name: "Delete Test".into(), official_website: "https://delete.example".into(), country: "US".into(), language: "en".into(), city_or_service_area: None }).unwrap();
        assert!(database.delete_lead(&lead.id, "Delete Test").is_err());
        database.set_lead_status(&lead.id, "rejected", Some("Not a management company")).unwrap();
        assert!(database.delete_lead(&lead.id, "wrong").is_err());
        database.delete_lead(&lead.id, "Delete Test").unwrap();
        assert!(database.list_leads().unwrap().is_empty());
    }
}
