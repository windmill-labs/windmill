-- A chat run from the flow editor's test panel is stored exactly like one from the
-- deployed flow, so the two were indistinguishable once written. Marking them lets the
-- lists tell a trial apart from a real conversation.
ALTER TABLE flow_conversation ADD COLUMN is_test BOOLEAN NOT NULL DEFAULT false;

-- Existing rows: a conversation whose messages came from a flowpreview run was a test.
-- Derived once here because the job is purged on retention, after which the origin of an
-- old conversation is unknowable.
--
-- Walked to the root job rather than matched directly: an existing message row never holds
-- the flow job itself. Only this migration's release starts storing it on the user row, and
-- the rows written before it point at the step that produced them — the AI agent's job for
-- an answer, the tool's own job for a tool call — whose kind is never 'flowpreview'.
--
-- `root_job` first, matching `get_root_job_id` (windmill-worker/src/common.rs): only it
-- reaches the top of the run. `flow_innermost_root_job` stops at the closest flow scope by
-- design, so an agent inside a subflow would land on that subflow's 'flow' row and the
-- conversation would read as deployed.
UPDATE flow_conversation c
SET is_test = true
WHERE EXISTS (
    SELECT 1 FROM flow_conversation_message m
    JOIN v2_job j ON j.id = m.job_id
    JOIN v2_job root
      ON root.id = coalesce(j.root_job, j.flow_innermost_root_job, j.parent_job, j.id)
    WHERE m.conversation_id = c.id AND root.kind = 'flowpreview'
);
