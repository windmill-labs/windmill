-- Add up migration script here
GRANT ALL ON v2_as_queue TO windmill_admin;
GRANT ALL ON v2_as_queue TO windmill_user;

GRANT ALL ON v2_as_completed_job TO windmill_admin;
GRANT ALL ON v2_as_completed_job TO windmill_user;
