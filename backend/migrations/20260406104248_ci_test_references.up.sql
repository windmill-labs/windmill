-- CI test references: maps test scripts to the items they test
CREATE TABLE ci_test_reference (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    test_script_path VARCHAR(510) NOT NULL,
    test_script_hash BIGINT NOT NULL,
    tested_item_path VARCHAR(510) NOT NULL,
    tested_item_kind VARCHAR(10) NOT NULL,
    PRIMARY KEY (workspace_id, test_script_path, tested_item_path, tested_item_kind)
);

-- Fast lookup: "which tests cover this item?" (used on deploy)
CREATE INDEX idx_ci_test_ref_tested_item
    ON ci_test_reference (workspace_id, tested_item_path, tested_item_kind);

-- Fast lookup: "which items does this test cover?" (used on test script update)
CREATE INDEX idx_ci_test_ref_test_script
    ON ci_test_reference (workspace_id, test_script_path);

GRANT ALL ON ci_test_reference TO windmill_user;
GRANT ALL ON ci_test_reference TO windmill_admin;

ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'ci_test';
