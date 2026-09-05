mod database;
mod error;
mod leads;
mod review;
mod contacts;
mod exports;
mod settings;
mod drafts;

use database::{portable_root, Database};
use leads::{CreateLeadInput, DashboardCounts, LeadSummary, UpdateLeadInput};
use review::{AddEvidenceInput, ReviewData};
use contacts::{AddContactInput, ContactRecord};
use std::collections::BTreeMap;
use drafts::{DraftSeed, OutreachDraft, SaveDraftInput};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn healthcheck(database: tauri::State<'_, Database>) -> Result<String, String> {
    database
        .healthcheck()
        .map(|_| "ready".to_owned())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_lead(database: tauri::State<'_, Database>, input: CreateLeadInput) -> Result<LeadSummary, String> {
    database.create_lead(input).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_leads(database: tauri::State<'_, Database>) -> Result<Vec<LeadSummary>, String> {
    database.list_leads().map_err(|error| error.to_string())
}

#[tauri::command]
fn update_lead(database: tauri::State<'_, Database>, input: UpdateLeadInput) -> Result<LeadSummary, String> {
    database.update_lead(input).map_err(|error| error.to_string())
}

#[tauri::command]
fn dashboard_counts(database: tauri::State<'_, Database>) -> Result<DashboardCounts, String> {
    database.dashboard_counts().map_err(|error| error.to_string())
}

#[tauri::command]
fn set_lead_status(database: tauri::State<'_, Database>, id: String, status: String, reason: Option<String>) -> Result<(), String> {
    database.set_lead_status(&id, &status, reason.as_deref()).map_err(|error| error.to_string())
}

#[tauri::command]
fn add_evidence(database: tauri::State<'_, Database>, input: AddEvidenceInput) -> Result<(), String> {
    database.add_evidence(input).map_err(|error| error.to_string())
}

#[tauri::command]
fn add_note(database: tauri::State<'_, Database>, lead_id: String, body: String) -> Result<(), String> {
    database.add_note(&lead_id, &body).map_err(|error| error.to_string())
}

#[tauri::command]
fn review_data(database: tauri::State<'_, Database>, lead_id: String) -> Result<ReviewData, String> {
    database.review_data(&lead_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn add_contact(database: tauri::State<'_, Database>, input: AddContactInput) -> Result<(), String> {
    database.add_contact(input).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_contacts(database: tauri::State<'_, Database>, lead_id: String) -> Result<Vec<ContactRecord>, String> {
    database.contacts(&lead_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_settings(database: tauri::State<'_, Database>) -> Result<BTreeMap<String, String>, String> {
    database.settings().map_err(|error| error.to_string())
}

#[tauri::command]
fn update_setting(database: tauri::State<'_, Database>, key: String, value: String) -> Result<(), String> {
    database.update_setting(&key, &value).map_err(|error| error.to_string())
}

#[tauri::command]
fn create_backup(database: tauri::State<'_, Database>) -> Result<String, String> {
    database.create_backup().map_err(|error| error.to_string())
}

#[tauri::command]
fn export_leads(database: tauri::State<'_, Database>, format: String) -> Result<String, String> {
    database.export_leads(&format).map_err(|error| error.to_string())
}

#[tauri::command]
fn draft_seed(database: tauri::State<'_, Database>, lead_id: String) -> Result<DraftSeed, String> { database.draft_seed(&lead_id).map_err(|e| e.to_string()) }
#[tauri::command]
fn save_draft(database: tauri::State<'_, Database>, input: SaveDraftInput) -> Result<OutreachDraft, String> { database.save_draft(input).map_err(|e| e.to_string()) }
#[tauri::command]
fn latest_draft(database: tauri::State<'_, Database>, lead_id: String) -> Result<Option<OutreachDraft>, String> { database.latest_draft(&lead_id).map_err(|e| e.to_string()) }
#[tauri::command]
fn approve_draft(database: tauri::State<'_, Database>, id: String) -> Result<OutreachDraft, String> { database.approve_draft(&id).map_err(|e| e.to_string()) }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let database = Database::open()?;
            app.manage(database);
            let webview_data = portable_root()?.join("data").join("webview");
            std::fs::create_dir_all(&webview_data)?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("IsoPlan Lead Desk")
                .inner_size(1280.0, 820.0)
                .min_inner_size(1024.0, 680.0)
                .data_directory(webview_data)
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![healthcheck, create_lead, update_lead, list_leads, dashboard_counts, set_lead_status, add_evidence, add_note, review_data, add_contact, list_contacts, get_settings, update_setting, create_backup, export_leads, draft_seed, save_draft, latest_draft, approve_draft])
        .run(tauri::generate_context!())
        .expect("IsoPlan Lead Desk failed to start");
}
