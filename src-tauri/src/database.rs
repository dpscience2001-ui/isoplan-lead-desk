use std::{fs, path::PathBuf, sync::Mutex};

use rusqlite::Connection;
use crate::error::AppError;

const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_review_workspace.sql");
const MIGRATION_003: &str = include_str!("../migrations/003_settings_and_contacts.sql");
const MIGRATION_004: &str = include_str!("../migrations/004_outreach_workflow.sql");

pub struct Database {
    connection: Mutex<Connection>,
    #[allow(dead_code)]
    path: PathBuf,
}

impl Database {
    pub fn open() -> Result<Self, AppError> {
        // IsoPlan is intentionally portable: its database lives beside the
        // executable instead of silently writing to the Windows system drive.
        let directory = portable_root()?.join("data");
        fs::create_dir_all(&directory)?;
        let path = directory.join("isoplan-lead-desk.db");
        let mut connection = Connection::open(&path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Self::migrate(&mut connection)?;
        Ok(Self { connection: Mutex::new(connection), path })
    }

    fn migrate(connection: &mut Connection) -> Result<(), AppError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        for (version, sql) in [(1_i64, MIGRATION_001), (2_i64, MIGRATION_002), (3_i64, MIGRATION_003), (4_i64, MIGRATION_004)] {
            let applied: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                [version],
                |row| row.get(0),
            )?;
            if !applied {
                let transaction = connection.transaction()?;
                transaction.execute_batch(sql)?;
                transaction.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [version])?;
                transaction.commit()?;
            }
        }
        Ok(())
    }

    pub fn healthcheck(&self) -> Result<(), AppError> {
        let connection = self.connection()?;
        connection.query_row("SELECT 1", [], |_| Ok(()))?;
        Ok(())
    }

    pub(crate) fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.connection.lock().map_err(|_| AppError::DatabaseBusy)
    }
}

pub fn portable_root() -> Result<PathBuf, AppError> {
    let executable = std::env::current_exe().map_err(|_| AppError::AppDataUnavailable)?;
    executable
        .parent()
        .map(PathBuf::from)
        .ok_or(AppError::AppDataUnavailable)
}
