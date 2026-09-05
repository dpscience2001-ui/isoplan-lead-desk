use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("application data directory is unavailable")]
    AppDataUnavailable,
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV export error: {0}")]
    Csv(#[from] csv::Error),
    #[error("JSON export error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the local database is temporarily busy")]
    DatabaseBusy,
    #[error("invalid lead: {0}")]
    InvalidLead(String),
    #[error("a lead with this official website already exists")]
    DuplicateLead,
    #[error("this business is on the do-not-contact list")]
    DoNotContact,
}
