-- Add up migration script here
CREATE TABLE instance_group (name VARCHAR(255) PRIMARY KEY, summary VARCHAR(2000), external_id VARCHAR(1000));
CREATE TABLE email_to_igroup (email VARCHAR(255), igroup VARCHAR(255), PRIMARY KEY (email, igroup));