use std::{fs, path::PathBuf};

use chrono::Utc;
use serde::Serialize;
use rusqlite::{backup::Progress, DatabaseName, OpenFlags};

use crate::{database::Database, error::AppError};

#[derive(Serialize)]
struct ExportLead {
    id: String,
    company_name: String,
    official_website: String,
    country: String,
    language: String,
    city_or_service_area: Option<String>,
    status: String,
    qualification_score: Option<i64>,
    confidence_level: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_name: String,
    pub size_bytes: u64,
}

impl Database {
    pub fn create_backup(&self) -> Result<String, AppError> {
        let directory = self.data_directory()?.join("backups");
        fs::create_dir_all(&directory)?;
        let path = timestamped_path(&directory, "isoplan-backup", "db");
        let connection = self.connection()?;
        connection.execute("VACUUM INTO ?1", [path.to_string_lossy().as_ref()])?;
        let retention = connection.query_row("SELECT value FROM settings WHERE key = 'backup_retention_count'", [], |row| row.get::<_, String>(0))?.parse::<usize>().unwrap_or(10).clamp(1, 30);
        drop(connection);
        prune_backups(&directory, retention)?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub fn list_backups(&self) -> Result<Vec<BackupInfo>, AppError> {
        let directory = self.data_directory()?.join("backups");
        fs::create_dir_all(&directory)?;
        let mut backups = fs::read_dir(directory)?.filter_map(Result::ok).filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_none_or(|value| value != "db") { return None; }
            Some(BackupInfo { file_name: entry.file_name().to_string_lossy().into_owned(), size_bytes: entry.metadata().ok()?.len() })
        }).collect::<Vec<_>>();
        backups.sort_by(|a,b| b.file_name.cmp(&a.file_name));
        Ok(backups)
    }

    pub fn restore_backup(&self, file_name: &str, confirmation: &str) -> Result<(), AppError> {
        if confirmation != "RESTORE" { return Err(AppError::InvalidLead("type RESTORE to confirm".into())); }
        if file_name.contains('/') || file_name.contains('\\') || !file_name.starts_with("isoplan-backup-") || !file_name.ends_with(".db") {
            return Err(AppError::InvalidLead("invalid backup selection".into()));
        }
        let directory = self.data_directory()?.join("backups");
        let canonical_directory = fs::canonicalize(&directory)?;
        let path = fs::canonicalize(directory.join(file_name))?;
        if path.parent() != Some(canonical_directory.as_path()) { return Err(AppError::InvalidLead("backup must be inside the portable backup directory".into())); }
        let source = rusqlite::Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let integrity: String = source.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" { return Err(AppError::InvalidLead("backup failed SQLite integrity validation".into())); }
        let version: i64 = source.query_row("SELECT COALESCE(MAX(version),0) FROM schema_migrations", [], |row| row.get(0))?;
        if version < 1 || version > 5 { return Err(AppError::InvalidLead("backup schema version is unsupported".into())); }
        drop(source);
        self.create_backup()?;
        let mut connection = self.connection()?;
        connection.restore(DatabaseName::Main, &path, None::<fn(Progress)>)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(())
    }

    pub fn export_leads(&self, format: &str) -> Result<String, AppError> {
        if !matches!(format, "csv" | "json") { return Err(AppError::InvalidLead("export format must be CSV or JSON".into())); }
        let directory = self.data_directory()?.join("exports");
        fs::create_dir_all(&directory)?;
        let path = timestamped_path(&directory, "isoplan-leads", format);
        let leads = self.export_rows()?;
        if format == "json" {
            let file = fs::File::create(&path)?;
            serde_json::to_writer_pretty(file, &leads)?;
        } else {
            let mut writer = csv::Writer::from_path(&path)?;
            writer.write_record(["Lead ID", "Company name", "Official website", "Country", "Language", "City or service area", "Status", "Qualification score", "Confidence", "Created", "Updated"])?;
            for lead in leads {
                writer.write_record([
                    safe_csv(&lead.id), safe_csv(&lead.company_name), safe_csv(&lead.official_website), safe_csv(&lead.country),
                    safe_csv(&lead.language), safe_csv(lead.city_or_service_area.as_deref().unwrap_or("")), safe_csv(&lead.status),
                    lead.qualification_score.map(|v| v.to_string()).unwrap_or_default(), safe_csv(lead.confidence_level.as_deref().unwrap_or("")),
                    safe_csv(&lead.created_at), safe_csv(&lead.updated_at),
                ])?;
            }
            writer.flush()?;
        }
        Ok(path.to_string_lossy().into_owned())
    }

    fn export_rows(&self) -> Result<Vec<ExportLead>, AppError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT id, company_name, official_website, country, language, city_or_service_area, status, qualification_score, confidence_level, created_at, updated_at FROM leads ORDER BY created_at")?;
        let leads = statement
            .query_map([], |row| Ok(ExportLead { id: row.get(0)?, company_name: row.get(1)?, official_website: row.get(2)?, country: row.get(3)?, language: row.get(4)?, city_or_service_area: row.get(5)?, status: row.get(6)?, qualification_score: row.get(7)?, confidence_level: row.get(8)?, created_at: row.get(9)?, updated_at: row.get(10)? }))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(leads)
    }
}

fn timestamped_path(directory: &std::path::Path, prefix: &str, extension: &str) -> PathBuf {
    directory.join(format!("{prefix}-{}-{}.{}", Utc::now().format("%Y%m%d-%H%M%S-%3f"), uuid::Uuid::new_v4().simple(), extension))
}

fn safe_csv(value: &str) -> String {
    if matches!(value.trim_start().chars().next(), Some('=' | '+' | '-' | '@')) { format!("'{value}") } else { value.to_owned() }
}

fn prune_backups(directory: &std::path::Path, keep: usize) -> Result<(), AppError> {
    let mut files = fs::read_dir(directory)?.filter_map(Result::ok).filter(|entry| entry.path().extension().is_some_and(|extension| extension == "db")).collect::<Vec<_>>();
    files.sort_by_key(|entry| std::cmp::Reverse(entry.file_name()));
    for entry in files.into_iter().skip(keep) { fs::remove_file(entry.path())?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leads::CreateLeadInput;
    use uuid::Uuid;

    #[test]
    fn restore_validates_and_restores_portable_backup() {
        let root = std::env::temp_dir().join(format!("isoplan-backup-test-{}", Uuid::new_v4()));
        let database = Database::open_path(root.join("isoplan.db")).unwrap();
        let lead = database.create_lead(CreateLeadInput { company_name:"Before Backup".into(), official_website:"https://backup.test".into(), country:"US".into(), language:"en".into(), city_or_service_area:None }).unwrap();
        let backup_path = database.create_backup().unwrap();
        database.set_lead_status(&lead.id, "rejected", Some("test")).unwrap();
        database.delete_lead(&lead.id, "Before Backup").unwrap();
        let file_name = std::path::Path::new(&backup_path).file_name().unwrap().to_string_lossy().into_owned();
        assert!(database.restore_backup(&file_name, "wrong").is_err());
        database.restore_backup(&file_name, "RESTORE").unwrap();
        assert_eq!(database.list_leads().unwrap().len(), 1);
    }
}
