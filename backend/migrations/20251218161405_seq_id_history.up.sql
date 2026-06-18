-- Add up migration script here
GRANT ALL ON SEQUENCE  folder_permission_history_id_seq  TO windmill_user;
GRANT ALL ON SEQUENCE  folder_permission_history_id_seq  TO windmill_admin;

GRANT ALL ON SEQUENCE  group_permission_history_id_seq  TO windmill_user;
GRANT ALL ON SEQUENCE  group_permission_history_id_seq  TO windmill_admin;
