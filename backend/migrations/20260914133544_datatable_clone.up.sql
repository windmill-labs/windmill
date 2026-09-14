-- A database created to hold a copy of a data table, until a fork takes it.
--
-- Creating the copy and creating the fork are separate requests, and the fork request names the
-- database it takes. Without a record of what each copy was made from and for whom, a fork could
-- name any `wm_fork_*` database — another workspace's copy, full of rows its members were never
-- given. A fork takes one only when it was copied from the data table it forks, by the user
-- creating the fork, and no fork has taken it yet.
CREATE TABLE datatable_clone (
    dbname VARCHAR(63) PRIMARY KEY,
    source_workspace_id VARCHAR(50) NOT NULL,
    source_datatable VARCHAR(255) NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    -- `schema_only` or `schema_and_data`. NULL for a database created empty, to be filled by a
    -- separate import.
    fork_behavior VARCHAR(20),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- The fork that took it. NULL while nothing has.
    claimed_by_workspace_id VARCHAR(50)
);

GRANT ALL ON datatable_clone TO windmill_user;
GRANT ALL ON datatable_clone TO windmill_admin;
