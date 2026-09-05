ALTER TABLE outreach_drafts ADD COLUMN contact_id TEXT REFERENCES contacts(id) ON DELETE SET NULL;

CREATE TABLE outreach_templates (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('initial', 'follow_up')),
    language TEXT NOT NULL CHECK (language IN ('en', 'fr')),
    subject_template TEXT NOT NULL,
    body_template TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO outreach_templates(id, name, kind, language, subject_template, body_template, is_default) VALUES
('initial-en', 'Concise introduction', 'initial', 'en', 'A clearer way to present {{company}} layouts', 'Hello,\n\nI’m Diptarko, a 3D visualizer specializing in furnished 3D floor plans.\n\n{{observation}}\n\nA clear 3D floor plan can help guests understand how rooms and units connect before booking. You can see my verified work and client feedback on Fiverr: {{fiverr_url}}\n\nWould this be useful for any of your properties?\n\n{{opt_out}}\n\nBest,\n{{sender_name}}', 1),
('initial-fr', 'Présentation concise', 'initial', 'fr', 'Une présentation plus claire des espaces de {{company}}', 'Bonjour,\n\nJe m’appelle Diptarko et je réalise des plans 3D meublés pour la présentation de biens immobiliers.\n\n{{observation}}\n\nUn plan 3D clair peut aider les voyageurs à comprendre l’agencement des pièces et des différents espaces avant de réserver. Vous pouvez consulter mes réalisations vérifiées et les avis de mes clients sur Fiverr : {{fiverr_url}}\n\nCe type de visuel pourrait-il être utile pour certains de vos biens ?\n\n{{opt_out}}\n\nBien cordialement,\n{{sender_name}}', 1);

CREATE INDEX outreach_drafts_lead_kind_index ON outreach_drafts(lead_id, kind, updated_at DESC);

