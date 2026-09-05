INSERT OR IGNORE INTO settings(key, value) VALUES
    ('sender_name', 'Diptarko Mukherjee'),
    ('sender_business_name', 'IsoPlan Studio'),
    ('sender_postal_address', ''),
    ('opt_out_line_en', 'If this is not relevant, just let me know and I will not contact you again.'),
    ('opt_out_line_fr', 'Si cela ne vous concerne pas, dites-le-moi simplement et je ne vous recontacterai pas.'),
    ('backup_retention_count', '10');

CREATE UNIQUE INDEX contacts_lead_type_value_unique ON contacts(lead_id, type, value);

