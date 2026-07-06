-- Cosmetic display label for a dev workspace: NULL/'dev' render as "dev", 'staging' renders as "stg".
-- Only meaningful when is_dev_workspace = true; changes nothing about behavior (locking, promote and
-- compare all key off is_dev_workspace / parent_workspace_id). The value is validated in the handler.
ALTER TABLE workspace ADD COLUMN dev_workspace_label VARCHAR;
