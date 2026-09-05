use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{database::Database, error::AppError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddEvidenceInput {
    pub lead_id: String,
    pub classification: String,
    pub category: String,
    pub claim: String,
    pub supports_qualification: i8,
    pub confidence: String,
    pub source_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceClaim {
    pub id: String,
    pub classification: String,
    pub category: String,
    pub claim: String,
    pub supports_qualification: i8,
    pub confidence: String,
    pub source_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeadNote {
    pub id: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewData {
    pub evidence: Vec<EvidenceClaim>,
    pub notes: Vec<LeadNote>,
}

const CATEGORIES: &[&str] = &[
    "relevant_business_type", "portfolio_size", "public_business_contact",
    "no_floor_plan_found", "weak_2d_floor_plan", "multi_unit", "premium_marketing",
    "geographic_match", "repeat_work_likelihood", "excellent_3d_floor_plans",
    "individual_host", "no_lawful_contact",
];

impl Database {
    pub fn add_evidence(&self, input: AddEvidenceInput) -> Result<(), AppError> {
        if !matches!(input.classification.as_str(), "verified_fact" | "extracted_statement" | "ai_inference" | "missing_information" | "user_entered") {
            return Err(AppError::InvalidLead("unknown evidence classification".into()));
        }
        if !CATEGORIES.contains(&input.category.as_str()) {
            return Err(AppError::InvalidLead("unknown scoring category".into()));
        }
        if !matches!(input.supports_qualification, -1..=1) {
            return Err(AppError::InvalidLead("evidence direction must be -1, 0, or 1".into()));
        }
        if !matches!(input.confidence.as_str(), "low" | "medium" | "high") {
            return Err(AppError::InvalidLead("unknown evidence confidence".into()));
        }
        let claim = input.claim.trim();
        if claim.len() < 5 || claim.len() > 2_000 {
            return Err(AppError::InvalidLead("evidence must be 5–2,000 characters".into()));
        }
        if input.classification != "user_entered" && input.classification != "missing_information" && input.source_url.is_none() {
            return Err(AppError::InvalidLead("this evidence type requires an exact source URL".into()));
        }

        let normalized_source = input.source_url.as_deref().map(validate_source_url).transpose()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let source_id = if let Some(url) = normalized_source {
            let existing: Option<String> = transaction.query_row(
                "SELECT id FROM sources WHERE lead_id = ?1 AND url = ?2", params![input.lead_id, url], |row| row.get(0),
            ).optional()?;
            if let Some(id) = existing { Some(id) } else {
                let id = Uuid::new_v4().to_string();
                transaction.execute(
                    "INSERT INTO sources(id, lead_id, url, retrieved_at, retrieval_status) VALUES(?1, ?2, ?3, CURRENT_TIMESTAMP, 'user_supplied')",
                    params![id, input.lead_id, url],
                )?;
                Some(id)
            }
        } else { None };
        let evidence_id = Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO evidence_claims(id, lead_id, source_id, classification, category, claim, supports_qualification, confidence)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![evidence_id, input.lead_id, source_id, input.classification, input.category, claim, input.supports_qualification, input.confidence],
        )?;
        if matches!(input.category.as_str(), "no_floor_plan_found" | "weak_2d_floor_plan" | "multi_unit") && input.supports_qualification == 1 {
            transaction.execute(
                "INSERT INTO opportunity_signals(id, lead_id, signal_type, evidence_claim_id, strength) VALUES(?1, ?2, ?3, ?4, ?5)",
                params![Uuid::new_v4().to_string(), input.lead_id, input.category, evidence_id, confidence_strength(&input.confidence)],
            )?;
        }
        recalculate_score(&transaction, &input.lead_id)?;
        transaction.execute(
            "INSERT INTO timeline_events(id, lead_id, event_type, detail) VALUES(?1, ?2, 'evidence_added', ?3)",
            params![Uuid::new_v4().to_string(), input.lead_id, claim],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn add_note(&self, lead_id: &str, body: &str) -> Result<(), AppError> {
        let body = body.trim();
        if body.is_empty() || body.len() > 4_000 { return Err(AppError::InvalidLead("note must be 1–4,000 characters".into())); }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute("INSERT INTO notes(id, lead_id, body) VALUES(?1, ?2, ?3)", params![Uuid::new_v4().to_string(), lead_id, body])?;
        transaction.execute("INSERT INTO timeline_events(id, lead_id, event_type, detail) VALUES(?1, ?2, 'note_added', 'User note added')", params![Uuid::new_v4().to_string(), lead_id])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn review_data(&self, lead_id: &str) -> Result<ReviewData, AppError> {
        let connection = self.connection()?;
        let mut evidence_statement = connection.prepare(
            "SELECT e.id, e.classification, e.category, e.claim, e.supports_qualification, e.confidence, s.url, e.created_at
             FROM evidence_claims e LEFT JOIN sources s ON s.id = e.source_id WHERE e.lead_id = ?1 ORDER BY e.created_at DESC",
        )?;
        let evidence = evidence_statement.query_map([lead_id], |row| Ok(EvidenceClaim {
            id: row.get(0)?, classification: row.get(1)?, category: row.get(2)?, claim: row.get(3)?,
            supports_qualification: row.get(4)?, confidence: row.get(5)?, source_url: row.get(6)?, created_at: row.get(7)?,
        }))?.collect::<Result<Vec<_>, _>>()?;
        drop(evidence_statement);
        let mut notes_statement = connection.prepare("SELECT id, body, created_at FROM notes WHERE lead_id = ?1 ORDER BY created_at DESC")?;
        let notes = notes_statement.query_map([lead_id], |row| Ok(LeadNote { id: row.get(0)?, body: row.get(1)?, created_at: row.get(2)? }))?.collect::<Result<Vec<_>, _>>()?;
        Ok(ReviewData { evidence, notes })
    }
}

fn validate_source_url(value: &str) -> Result<String, AppError> {
    let mut url = Url::parse(value.trim()).map_err(|_| AppError::InvalidLead("source must be a valid URL".into()))?;
    if !matches!(url.scheme(), "http" | "https") { return Err(AppError::InvalidLead("source must use HTTP or HTTPS".into())); }
    url.set_fragment(None);
    Ok(url.to_string())
}

fn confidence_strength(value: &str) -> i64 { match value { "high" => 3, "medium" => 2, _ => 1 } }
fn category_weight(category: &str) -> i64 { match category {
    "relevant_business_type" => 20, "portfolio_size" => 15, "public_business_contact" => 10,
    "no_floor_plan_found" | "weak_2d_floor_plan" | "multi_unit" => 15,
    "premium_marketing" | "geographic_match" => 5, "repeat_work_likelihood" => 10,
    "excellent_3d_floor_plans" | "no_lawful_contact" => 25, "individual_host" => 40, _ => 0,
} }

fn recalculate_score(transaction: &rusqlite::Transaction<'_>, lead_id: &str) -> Result<(), rusqlite::Error> {
    let mut statement = transaction.prepare("SELECT category, supports_qualification, confidence FROM evidence_claims WHERE lead_id = ?1")?;
    let rows = statement.query_map([lead_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))?;
    let mut strongest = std::collections::HashMap::<String, i64>::new();
    let mut count = 0; let mut high = 0;
    for row in rows { let (category, direction, confidence) = row?; count += 1; if confidence == "high" { high += 1; } strongest.entry(category).and_modify(|current| { if direction.abs() >= current.abs() { *current = direction; } }).or_insert(direction); }
    drop(statement);
    let score = strongest.iter().map(|(category, direction)| category_weight(category) * direction).sum::<i64>().clamp(0, 100);
    let confidence = if count >= 5 && high >= 3 { "high" } else if count >= 3 { "medium" } else { "low" };
    let explanation = strongest.iter().map(|(category, direction)| format!("{}: {:+}", category.replace('_', " "), category_weight(category) * direction)).collect::<Vec<_>>().join("; ");
    transaction.execute("UPDATE leads SET qualification_score = ?2, confidence_level = ?3, score_explanation = ?4, updated_at = CURRENT_TIMESTAMP WHERE id = ?1", params![lead_id, score, confidence, explanation])?;
    Ok(())
}
