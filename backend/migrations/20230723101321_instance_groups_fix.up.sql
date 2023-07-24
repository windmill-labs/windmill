-- Add up migration script here
GRANT ALL PRIVILEGES ON TABLE instance_group TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE instance_group TO windmill_user;
GRANT ALL PRIVILEGES ON TABLE email_to_igroup TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE email_to_igroup TO windmill_user;