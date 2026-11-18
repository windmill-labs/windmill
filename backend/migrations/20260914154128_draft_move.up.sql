-- Where an item's drafts went when it moved, so a draft save still addressed to the
-- old path (an editor left open across the move, a chat, the CLI) lands on the moved
-- draft instead of starting a new item there. `email` NULL records a deployed item's
-- move and applies to every user; set, it records that user's draft-only move.
CREATE TABLE draft_move (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    typ DRAFT_KIND NOT NULL,
    old_path VARCHAR(255) NOT NULL,
    new_path VARCHAR(255) NOT NULL,
    email VARCHAR(255)
);

CREATE INDEX draft_move_old_path_idx ON draft_move (workspace_id, typ, old_path);

GRANT ALL ON draft_move TO windmill_user;
GRANT ALL ON draft_move TO windmill_admin;
