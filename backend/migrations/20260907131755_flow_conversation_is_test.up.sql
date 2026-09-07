-- A chat run from the flow editor's test panel is stored exactly like one from the
-- deployed flow, so the two were indistinguishable once written. Marking them lets the
-- lists tell a trial apart from a real conversation.
ALTER TABLE flow_conversation ADD COLUMN is_test BOOLEAN NOT NULL DEFAULT false;

-- Existing rows: a conversation whose messages came from a flowpreview job was a test.
-- Derived once here because the job is purged on retention, after which the origin of an
-- old conversation is unknowable.
UPDATE flow_conversation c
SET is_test = true
WHERE EXISTS (
    SELECT 1 FROM flow_conversation_message m
    JOIN v2_job j ON j.id = m.job_id
    WHERE m.conversation_id = c.id AND j.kind = 'flowpreview'
);
