CREATE TABLE notes (
    id TEXT PRIMARY KEY NOT NULL,
    lead_id TEXT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    body TEXT NOT NULL CHECK (length(body) BETWEEN 1 AND 4000),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE tags (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    color TEXT NOT NULL DEFAULT '#667c73'
);

CREATE TABLE lead_tags (
    lead_id TEXT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (lead_id, tag_id)
);

CREATE TABLE opportunity_signals (
    id TEXT PRIMARY KEY NOT NULL,
    lead_id TEXT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    signal_type TEXT NOT NULL CHECK (signal_type IN ('no_floor_plan_found','weak_2d_floor_plan','multi_unit')),
    evidence_claim_id TEXT REFERENCES evidence_claims(id) ON DELETE SET NULL,
    strength INTEGER NOT NULL CHECK (strength BETWEEN 1 AND 3),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX notes_lead_index ON notes(lead_id, created_at DESC);
CREATE INDEX evidence_lead_index ON evidence_claims(lead_id, category);
CREATE INDEX contacts_lead_index ON contacts(lead_id);

