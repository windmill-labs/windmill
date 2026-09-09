-- `draft`'s BIGSERIAL id arrived in 20260528143710, and the draft upsert now runs
-- under `SET LOCAL ROLE windmill_user`, so it calls nextval on this sequence as
-- that role. The only thing granting it is the ALTER DEFAULT PRIVILEGES in
-- 20250205131523, whose DO block swallows failures, so an instance where that
-- block errored has the table grant but not the sequence one.
GRANT ALL ON SEQUENCE draft_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE draft_id_seq TO windmill_admin;
