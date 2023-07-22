-- Add up migration script here
CREATE TABLE instance_group (name VARCHAR(255) PRIMARY KEY, summary VARCHAR(2000));
CREATE TABLE email_to_igroup (email VARCHAR(255), igroup VARCHAR(255) NOT NULL, PRIMARY KEY (email, igroup));

GRANT ALL PRIVILEGES ON TABLE instance_group TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE instance_group TO windmill_user;
GRANT ALL PRIVILEGES ON TABLE email_to_igroup TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE email_to_igroup TO windmill_user;