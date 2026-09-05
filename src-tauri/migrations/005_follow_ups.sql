CREATE TABLE follow_ups (
    id TEXT PRIMARY KEY NOT NULL,
    lead_id TEXT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL DEFAULT 1 CHECK (sequence_number IN (1, 2)),
    due_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'due' CHECK (status IN ('due', 'snoozed', 'completed', 'cancelled')),
    draft_id TEXT REFERENCES outreach_drafts(id) ON DELETE SET NULL,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(lead_id, sequence_number)
);

CREATE INDEX follow_ups_due_index ON follow_ups(status, due_at);

