use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{database::Database, error::AppError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDraftInput {
    pub id: Option<String>,
    pub lead_id: String,
    pub contact_id: String,
    pub language: String,
    pub subject: String,
    pub body: String,
    pub english_translation: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutreachDraft {
    pub id: String,
    pub lead_id: String,
    pub contact_id: Option<String>,
    pub recipient: Option<String>,
    pub language: String,
    pub subject: String,
    pub body: String,
    pub english_translation: Option<String>,
    pub approved_at: Option<String>,
    pub gmail_opened_at: Option<String>,
    pub sent_confirmed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftSeed {
    pub subject: String,
    pub body: String,
}

impl Database {
    pub fn draft_seed(&self, lead_id: &str) -> Result<DraftSeed, AppError> {
        let connection = self.connection()?;
        let (company, language, blocked): (String, String, bool) = connection.query_row("SELECT company_name, language, do_not_contact FROM leads WHERE id=?1", [lead_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        if blocked { return Err(AppError::DoNotContact); }
        let observation: String = connection.query_row(
            "SELECT claim FROM evidence_claims WHERE lead_id=?1 AND supports_qualification=1 AND classification != 'ai_inference' ORDER BY CASE confidence WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END, created_at DESC LIMIT 1",
            [lead_id], |row| row.get(0),
        ).optional()?.ok_or_else(|| AppError::InvalidLead("add at least one sourced, positive, non-AI evidence item before drafting".into()))?;
        let (subject, template): (String, String) = connection.query_row("SELECT subject_template, body_template FROM outreach_templates WHERE kind='initial' AND language=?1 AND is_default=1 LIMIT 1", [&language], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let setting = |key: &str| connection.query_row("SELECT value FROM settings WHERE key=?1", [key], |row| row.get::<_, String>(0));
        let fiverr = setting("fiverr_gig_url")?; let sender = setting("sender_name")?;
        let opt_out = setting(if language == "fr" { "opt_out_line_fr" } else { "opt_out_line_en" })?;
        let replace = |value: String| value.replace("{{company}}", &company).replace("{{observation}}", &observation).replace("{{fiverr_url}}", &fiverr).replace("{{sender_name}}", &sender).replace("{{opt_out}}", &opt_out);
        Ok(DraftSeed { subject: replace(subject), body: replace(template) })
    }

    pub fn save_draft(&self, input: SaveDraftInput) -> Result<OutreachDraft, AppError> {
        if !matches!(input.language.as_str(), "en" | "fr") { return Err(AppError::InvalidLead("draft language is invalid".into())); }
        let subject = input.subject.trim(); let body = input.body.trim();
        if subject.len() < 3 || subject.len() > 150 || body.len() < 20 || body.len() > 10_000 { return Err(AppError::InvalidLead("draft subject or body length is invalid".into())); }
        if subject.to_ascii_lowercase().starts_with("re:") { return Err(AppError::InvalidLead("an initial email must not pretend to be a reply".into())); }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let blocked: bool = transaction.query_row("SELECT do_not_contact FROM leads WHERE id=?1", [&input.lead_id], |row| row.get(0))?;
        if blocked { return Err(AppError::DoNotContact); }
        let contact_type: String = transaction.query_row("SELECT type FROM contacts WHERE id=?1 AND lead_id=?2", params![input.contact_id, input.lead_id], |row| row.get(0))?;
        if !matches!(contact_type.as_str(), "role_email" | "named_email") { return Err(AppError::InvalidLead("Gmail drafts require a published business email".into())); }
        let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        transaction.execute(
            "INSERT INTO outreach_drafts(id, lead_id, contact_id, kind, language, subject, body, english_translation)
             VALUES(?1,?2,?3,'initial',?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET contact_id=excluded.contact_id, language=excluded.language, subject=excluded.subject,
             body=excluded.body, english_translation=excluded.english_translation, approved_at=NULL,
             gmail_opened_at=NULL, sent_confirmed_at=NULL, updated_at=CURRENT_TIMESTAMP",
            params![id, input.lead_id, input.contact_id, input.language, subject, body, input.english_translation],
        )?;
        transaction.execute("UPDATE leads SET status='draft_ready', updated_at=CURRENT_TIMESTAMP WHERE id=?1", [&input.lead_id])?;
        transaction.execute("INSERT INTO timeline_events(id,lead_id,event_type,detail) VALUES(?1,?2,'draft_saved','Initial outreach draft saved for review')", params![Uuid::new_v4().to_string(), input.lead_id])?;
        transaction.commit()?;
        drop(connection);
        self.get_draft(&id)
    }

    pub fn latest_draft(&self, lead_id: &str) -> Result<Option<OutreachDraft>, AppError> {
        let connection = self.connection()?;
        let id: Option<String> = connection.query_row("SELECT id FROM outreach_drafts WHERE lead_id=?1 AND kind='initial' ORDER BY updated_at DESC LIMIT 1", [lead_id], |row| row.get(0)).optional()?;
        drop(connection);
        id.map(|value| self.get_draft(&value)).transpose()
    }

    pub fn approve_draft(&self, id: &str) -> Result<OutreachDraft, AppError> {
        let mut connection = self.connection()?; let transaction = connection.transaction()?;
        let (lead_id, blocked): (String, bool) = transaction.query_row("SELECT d.lead_id, l.do_not_contact FROM outreach_drafts d JOIN leads l ON l.id=d.lead_id WHERE d.id=?1", [id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        if blocked { return Err(AppError::DoNotContact); }
        transaction.execute("UPDATE outreach_drafts SET approved_at=CURRENT_TIMESTAMP, updated_at=CURRENT_TIMESTAMP WHERE id=?1", [id])?;
        transaction.execute("UPDATE leads SET status='approved', updated_at=CURRENT_TIMESTAMP WHERE id=?1", [&lead_id])?;
        transaction.execute("INSERT INTO timeline_events(id,lead_id,event_type,detail) VALUES(?1,?2,'draft_approved','User explicitly approved the complete draft')", params![Uuid::new_v4().to_string(), lead_id])?;
        transaction.commit()?;
        drop(connection);
        self.get_draft(id)
    }

    fn get_draft(&self, id: &str) -> Result<OutreachDraft, AppError> {
        let connection = self.connection()?;
        connection.query_row("SELECT d.id,d.lead_id,d.contact_id,c.value,d.language,d.subject,d.body,d.english_translation,d.approved_at,d.gmail_opened_at,d.sent_confirmed_at,d.updated_at FROM outreach_drafts d LEFT JOIN contacts c ON c.id=d.contact_id WHERE d.id=?1", [id], |row| Ok(OutreachDraft { id:row.get(0)?, lead_id:row.get(1)?, contact_id:row.get(2)?, recipient:row.get(3)?, language:row.get(4)?, subject:row.get(5)?, body:row.get(6)?, english_translation:row.get(7)?, approved_at:row.get(8)?, gmail_opened_at:row.get(9)?, sent_confirmed_at:row.get(10)?, updated_at:row.get(11)? })).map_err(Into::into)
    }
}
