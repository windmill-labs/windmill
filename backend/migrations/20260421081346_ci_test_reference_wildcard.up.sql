-- CI test references: flag rows whose tested_item_path contains a glob `*`
-- so wildcard matching only scans those rows (via partial index) while exact
-- paths keep using the primary (workspace_id, tested_item_path, tested_item_kind) index.
ALTER TABLE ci_test_reference
    ADD COLUMN has_wildcard BOOLEAN
    GENERATED ALWAYS AS (tested_item_path LIKE '%*%') STORED;

CREATE INDEX idx_ci_test_ref_wildcards
    ON ci_test_reference (workspace_id, tested_item_kind)
    WHERE has_wildcard;
