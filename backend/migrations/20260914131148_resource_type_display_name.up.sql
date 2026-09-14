-- The name a product goes by, beside the identifier a resource references: `gsheets` is
-- "Google Sheets". Null where nobody named the type; readers derive a label from the name.
ALTER TABLE resource_type ADD COLUMN display_name VARCHAR(100);

-- The names the hub carries today, so existing instances show them before any sync. Not
-- limited to admins: a workspace that copied a hub type keeps the type's name.
UPDATE resource_type SET display_name = v.display_name
FROM (VALUES
    ('bamboo_hr', 'BambooHR'),
    ('cacertificate', 'CA certificate'),
    ('deep_infra', 'DeepInfra'),
    ('gcal', 'Google Calendar'),
    ('gdocs', 'Google Docs'),
    ('gdrive', 'Google Drive'),
    ('gforms', 'Google Forms'),
    ('gsheets', 'Google Sheets'),
    ('gworkspace', 'Google Workspace'),
    ('sensortower', 'Sensor Tower'),
    ('snowflake_oauth', 'Snowflake (OAuth)'),
    ('their_stack', 'TheirStack')
) AS v(name, display_name)
WHERE resource_type.name = v.name AND resource_type.display_name IS NULL;
