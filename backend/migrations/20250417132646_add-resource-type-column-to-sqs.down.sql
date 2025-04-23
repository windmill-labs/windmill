-- Add down migration script here
ALTER TABLE sqs_trigger DROP COLUMN aws_auth_resource_type;
DROP TYPE IF EXISTS AWS_AUTH_RESOURCE_TYPE;