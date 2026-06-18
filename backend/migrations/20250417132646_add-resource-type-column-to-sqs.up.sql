-- Add up migration script here
CREATE TYPE AWS_AUTH_RESOURCE_TYPE AS ENUM ('oidc', 'credentials');
ALTER TABLE sqs_trigger 
    ADD COLUMN aws_auth_resource_type AWS_AUTH_RESOURCE_TYPE DEFAULT 'credentials'::AWS_AUTH_RESOURCE_TYPE NOT NULL;