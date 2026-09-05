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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowUpSummary {
    pub id: String,
    pub lead_id: String,
    pub company_name: String,
    pub due_at: String,
    pub status: String,
    pub recipient: Option<String>,
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

    pub fn compose_url(&self, id: &str) -> Result<String, AppError> {
        let draft = self.get_draft(id)?;
        if draft.approved_at.is_none() { return Err(AppError::InvalidLead("approve the exact draft before opening Gmail".into())); }
        if draft.sent_confirmed_at.is_some() { return Err(AppError::InvalidLead("this draft is already confirmed sent".into())); }
        let recipient = draft.recipient.ok_or_else(|| AppError::InvalidLead("draft recipient is missing".into()))?;
        let mut url = url::Url::parse("https://mail.google.com/mail/").expect("static Gmail URL");
        url.query_pairs_mut().append_pair("view", "cm").append_pair("fs", "1").append_pair("to", &recipient).append_pair("su", &draft.subject).append_pair("body", &draft.body);
        Ok(url.to_string())
    }

    pub fn mark_gmail_opened(&self, id: &str) -> Result<(), AppError> {
        let connection = self.connection()?;
        let lead_id: String = connection.query_row("SELECT lead_id FROM outreach_drafts WHERE id=?1 AND approved_at IS NOT NULL", [id], |row| row.get(0))?;
        connection.execute("UPDATE outreach_drafts SET gmail_opened_at=CURRENT_TIMESTAMP WHERE id=?1", [id])?;
        connection.execute("UPDATE leads SET status='gmail_opened', updated_at=CURRENT_TIMESTAMP WHERE id=?1", [&lead_id])?;
        connection.execute("INSERT INTO timeline_events(id,lead_id,event_type,detail) VALUES(?1,?2,'gmail_opened','Prepared Gmail compose opened; sending not yet confirmed')", params![Uuid::new_v4().to_string(), lead_id])?;
        Ok(())
    }

    pub fn confirm_sent(&self, id: &str, sent: bool) -> Result<(), AppError> {
        let mut connection = self.connection()?; let transaction = connection.transaction()?;
        let lead_id: String = transaction.query_row("SELECT lead_id FROM outreach_drafts WHERE id=?1 AND gmail_opened_at IS NOT NULL", [id], |row| row.get(0))?;
        if sent {
            let delay: String = transaction.query_row("SELECT value FROM settings WHERE key='follow_up_delay_days'", [], |row| row.get(0))?;
            let modifier = format!("+{} days", delay.parse::<i64>().unwrap_or(5).clamp(1, 30));
            transaction.execute("UPDATE outreach_drafts SET sent_confirmed_at=COALESCE(sent_confirmed_at,CURRENT_TIMESTAMP) WHERE id=?1", [id])?;
            transaction.execute("UPDATE leads SET status='contacted', first_contacted_at=COALESCE(first_contacted_at,CURRENT_TIMESTAMP), follow_up_due_at=datetime('now',?2), updated_at=CURRENT_TIMESTAMP WHERE id=?1", params![lead_id, modifier])?;
            transaction.execute("INSERT INTO follow_ups(id,lead_id,sequence_number,due_at) VALUES(?1,?2,1,datetime('now',?3)) ON CONFLICT(lead_id,sequence_number) DO NOTHING", params![Uuid::new_v4().to_string(), lead_id, modifier])?;
            transaction.execute("INSERT INTO timeline_events(id,lead_id,event_type,detail) VALUES(?1,?2,'sent_confirmed','User confirmed the message was sent manually')", params![Uuid::new_v4().to_string(), lead_id])?;
        } else {
            transaction.execute("UPDATE leads SET status='approved', updated_at=CURRENT_TIMESTAMP WHERE id=?1", [&lead_id])?;
            transaction.execute("INSERT INTO timeline_events(id,lead_id,event_type,detail) VALUES(?1,?2,'send_not_confirmed','User confirmed the prepared message was not sent')", params![Uuid::new_v4().to_string(), lead_id])?;
        }
        transaction.commit()?; Ok(())
    }

    pub fn due_follow_ups(&self) -> Result<Vec<FollowUpSummary>, AppError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT f.id,f.lead_id,l.company_name,f.due_at,f.status,(SELECT c.value FROM outreach_drafts d JOIN contacts c ON c.id=d.contact_id WHERE d.lead_id=l.id AND d.kind='initial' ORDER BY d.updated_at DESC LIMIT 1) FROM follow_ups f JOIN leads l ON l.id=f.lead_id WHERE f.status IN ('due','snoozed') AND l.do_not_contact=0 ORDER BY f.due_at")?;
        let values = statement.query_map([], |row| Ok(FollowUpSummary { id:row.get(0)?, lead_id:row.get(1)?, company_name:row.get(2)?, due_at:row.get(3)?, status:row.get(4)?, recipient:row.get(5)? }))?.collect::<Result<Vec<_>,_>>()?;
        Ok(values)
    }

    pub fn snooze_follow_up(&self, id: &str, days: i64) -> Result<(), AppError> {
        if !(1..=30).contains(&days) { return Err(AppError::InvalidLead("snooze must be 1–30 days".into())); }
        let connection = self.connection()?; let modifier = format!("+{days} days");
        connection.execute("UPDATE follow_ups SET status='snoozed', due_at=datetime('now',?2) WHERE id=?1", params![id,modifier])?;
        Ok(())
    }

    pub fn mark_replied(&self, lead_id: &str) -> Result<(), AppError> {
        let mut connection=self.connection()?; let transaction=connection.transaction()?;
        let blocked: bool = transaction.query_row("SELECT do_not_contact FROM leads WHERE id=?1", [lead_id], |row| row.get(0))?;
        if blocked { return Err(AppError::DoNotContact); }
        transaction.execute("UPDATE leads SET status='replied', reply_status='replied', updated_at=CURRENT_TIMESTAMP WHERE id=?1", [lead_id])?;
        transaction.execute("UPDATE follow_ups SET status='cancelled' WHERE lead_id=?1 AND status IN ('due','snoozed')", [lead_id])?;
        transaction.execute("INSERT INTO timeline_events(id,lead_id,event_type,detail) VALUES(?1,?2,'reply_recorded','User recorded a reply; pending follow-ups cancelled')", params![Uuid::new_v4().to_string(),lead_id])?;
        transaction.commit()?; Ok(())
    }

    fn get_draft(&self, id: &str) -> Result<OutreachDraft, AppError> {
        let connection = self.connection()?;
        connection.query_row("SELECT d.id,d.lead_id,d.contact_id,c.value,d.language,d.subject,d.body,d.english_translation,d.approved_at,d.gmail_opened_at,d.sent_confirmed_at,d.updated_at FROM outreach_drafts d LEFT JOIN contacts c ON c.id=d.contact_id WHERE d.id=?1", [id], |row| Ok(OutreachDraft { id:row.get(0)?, lead_id:row.get(1)?, contact_id:row.get(2)?, recipient:row.get(3)?, language:row.get(4)?, subject:row.get(5)?, body:row.get(6)?, english_translation:row.get(7)?, approved_at:row.get(8)?, gmail_opened_at:row.get(9)?, sent_confirmed_at:row.get(10)?, updated_at:row.get(11)? })).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{contacts::AddContactInput, leads::CreateLeadInput, review::AddEvidenceInput};

    fn database() -> Database {
        Database::open_path(std::env::temp_dir().join(format!("isoplan-draft-test-{}.db", Uuid::new_v4()))).unwrap()
    }

    #[test]
    fn manual_send_confirmation_creates_follow_up() {
        let database = database();
        let lead = database.create_lead(CreateLeadInput { company_name:"Example Stays".into(), official_website:"https://example.test".into(), country:"GB".into(), language:"en".into(), city_or_service_area:None }).unwrap();
        database.add_evidence(AddEvidenceInput { lead_id:lead.id.clone(), classification:"verified_fact".into(), category:"relevant_business_type".into(), claim:"Manages holiday properties".into(), supports_qualification:1, confidence:"high".into(), source_url:Some("https://example.test/about".into()) }).unwrap();
        database.add_contact(AddContactInput { lead_id:lead.id.clone(), contact_type:"role_email".into(), value:"hello@example.test".into(), recipient_role:Some("Reservations".into()), source_url:"https://example.test/contact".into() }).unwrap();
        let contact_id = database.contacts(&lead.id).unwrap()[0].id.clone();
        let seed = database.draft_seed(&lead.id).unwrap();
        let draft = database.save_draft(SaveDraftInput { id:None, lead_id:lead.id.clone(), contact_id, language:"en".into(), subject:seed.subject, body:seed.body, english_translation:None }).unwrap();
        database.approve_draft(&draft.id).unwrap();
        assert!(database.compose_url(&draft.id).unwrap().starts_with("https://mail.google.com/"));
        database.mark_gmail_opened(&draft.id).unwrap();
        database.confirm_sent(&draft.id, true).unwrap();
        let follow_ups = database.due_follow_ups().unwrap();
        assert_eq!(follow_ups.len(), 1);
        assert_eq!(follow_ups[0].company_name, "Example Stays");
    }
}
