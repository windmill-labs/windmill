-- Re-establish the ON DELETE CASCADE foreign keys. Once the cascades were gone the explicit
-- delete paths may have left orphan rows (or none were created); purge any orphans first so
-- the constraints can be validated.
DELETE FROM dispatch_event WHERE producer_job_id NOT IN (SELECT id FROM v2_job);
DELETE FROM flow_conversation_message WHERE job_id IS NOT NULL AND job_id NOT IN (SELECT id FROM v2_job);
DELETE FROM zombie_job_counter WHERE job_id NOT IN (SELECT id FROM v2_job);

ALTER TABLE dispatch_event
    ADD CONSTRAINT dispatch_event_producer_job_id_fkey
    FOREIGN KEY (producer_job_id) REFERENCES v2_job(id) ON DELETE CASCADE;
ALTER TABLE flow_conversation_message
    ADD CONSTRAINT flow_conversation_message_job_id_fkey
    FOREIGN KEY (job_id) REFERENCES v2_job(id) ON DELETE CASCADE;
ALTER TABLE zombie_job_counter
    ADD CONSTRAINT zombie_job_counter_job_id_fkey
    FOREIGN KEY (job_id) REFERENCES v2_job(id) ON DELETE CASCADE;
