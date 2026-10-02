-- The flow run a row's turn belongs to: the job the user message started. Retention collects
-- rows by it, so a turn goes as one unit. `job_id` names the step or tool job that wrote the
-- row, which completes, and so expires, before the run does.
ALTER TABLE flow_conversation_message ADD COLUMN turn_job_id UUID;

UPDATE flow_conversation_message m
   SET turn_job_id = (
       SELECT u.job_id FROM flow_conversation_message u
        WHERE u.conversation_id = m.conversation_id
          AND u.message_type = 'user'
          AND u.created_seq <= m.created_seq
        ORDER BY u.created_seq DESC
        LIMIT 1
   );
