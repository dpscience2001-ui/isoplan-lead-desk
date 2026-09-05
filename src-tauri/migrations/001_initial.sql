CREATE TABLE leads (
    id TEXT PRIMARY KEY NOT NULL,
    company_name TEXT NOT NULL,
    official_website TEXT NOT NULL,
    normalized_domain TEXT NOT NULL,
    country TEXT NOT NULL CHECK (country IN ('GB', 'US', 'FR')),
    city_or_service_area TEXT,
    language TEXT NOT NULL CHECK (language IN ('en', 'fr')),
    business_type TEXT,
    estimated_property_count INTEGER,
    property_count_confidence TEXT CHECK (property_count_confidence IN ('low', 'medium', 'high')),
    qualification_score INTEGER CHECK (qualification_score BETWEEN 0 AND 100),
    confidence_level TEXT CHECK (confidence_level IN ('low', 'medium', 'high')),
    score_explanation TEXT,
    research_summary TEXT,
    user_notes TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'discovered' CHECK (status IN (
        'discovered','researching','needs_review','qualified','rejected','draft_ready',
        'approved','gmail_opened','contacted','follow_up_due','replied','interested',
        'not_interested','converted','do_not_contact','archived'
    )),
    first_contacted_at TEXT,
    follow_up_due_at TEXT,
    followed_up_at TEXT,
    reply_status TEXT,
    do_not_contact INTEGER NOT NULL DEFAULT 0 CHECK (do_not_contact IN (0, 1)),
    do_not_contact_reason TEXT,
    duplicate_fingerprint TEXT NOT NULL,
    user_score_override INTEGER CHECK (user_score_override BETWEEN 0 AND 100),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX leads_normalized_domain_unique ON leads(normalized_domain);
CREATE INDEX leads_status_index ON leads(status);
CREATE INDEX leads_follow_up_due_index ON leads(follow_up_due_at);

CREATE TABLE contacts (
    id TEXT PRIMARY KEY NOT NULL,
    lead_id TEXT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    type TEXT NOT NULL CHECK (type IN ('role_email', 'named_email', 'contact_form', 'phone')),
    value TEXT NOT NULL,
    recipient_role TEXT,
    source_url TEXT NOT NULL,
    is_preferred INTEGER NOT NULL DEFAULT 0 CHECK (is_preferred IN (0, 1)),
    caution_reason TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE sources (
    id TEXT PRIMARY KEY NOT NULL,
    lead_id TEXT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    page_title TEXT,
    retrieved_at TEXT NOT NULL,
    retrieval_status TEXT NOT NULL,
    content_hash TEXT,
    UNIQUE(lead_id, url)
);

CREATE TABLE evidence_claims (
    id TEXT PRIMARY KEY NOT NULL,
    lead_id TEXT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    source_id TEXT REFERENCES sources(id) ON DELETE SET NULL,
    classification TEXT NOT NULL CHECK (classification IN ('verified_fact','extracted_statement','ai_inference','missing_information','user_entered')),
    category TEXT NOT NULL,
    claim TEXT NOT NULL,
    supports_qualification INTEGER NOT NULL CHECK (supports_qualification IN (-1, 0, 1)),
    confidence TEXT NOT NULL CHECK (confidence IN ('low', 'medium', 'high')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE outreach_drafts (
    id TEXT PRIMARY KEY NOT NULL,
    lead_id TEXT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('initial', 'follow_up')),
    language TEXT NOT NULL CHECK (language IN ('en', 'fr')),
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    english_translation TEXT,
    approved_at TEXT,
    gmail_opened_at TEXT,
    sent_confirmed_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE timeline_events (
    id TEXT PRIMARY KEY NOT NULL,
    lead_id TEXT NOT NULL REFERENCES leads(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    detail TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE do_not_contact_entries (
    id TEXT PRIMARY KEY NOT NULL,
    normalized_value TEXT NOT NULL UNIQUE,
    value_type TEXT NOT NULL CHECK (value_type IN ('domain', 'email')),
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO settings(key, value) VALUES
    ('fiverr_gig_url', 'https://www.fiverr.com/diptarkreator/convert-2d-floor-plan-to-3-d-for-real-estate-and-airbnb-and-custom'),
    ('normal_lead_limit', '10'),
    ('follow_up_delay_days', '5'),
    ('second_follow_up_enabled', 'false');

