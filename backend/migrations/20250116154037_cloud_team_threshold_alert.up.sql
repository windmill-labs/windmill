CREATE TABLE IF NOT EXISTS cloud_workspace_settings (
    workspace_id VARCHAR(50) NOT NULL,
    threshold_alert_amount INT NULL,
    last_alert_sent TIMESTAMP NULL,
    PRIMARY KEY (workspace_id),
    FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE
)