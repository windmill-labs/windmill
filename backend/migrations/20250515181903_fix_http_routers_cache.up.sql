-- Add up migration script here
-- this makes sure that the first time nextval is called, 2 is returned
-- otherwise, `SELECT last_value from http_trigger_version_seq;` would return 1 before and after the first nextval call
-- which would not refresh the routers cache after the first create/update/delete
SELECT setval(                                  
    'http_trigger_version_seq',
    GREATEST((SELECT last_value FROM http_trigger_version_seq), 1),
    true
);