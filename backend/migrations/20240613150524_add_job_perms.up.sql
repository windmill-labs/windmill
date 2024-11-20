-- Add up migration script here
CREATE TABLE public.job_perms (
	job_id uuid NOT NULL,
	email varchar(255) NOT NULL,
	username varchar(50) NOT NULL,
	is_admin bool NOT NULL,
	is_operator bool NOT NULL,
	created_at timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL,
	workspace_id varchar(50) NOT NULL,
	groups _text NOT NULL,
	folders _jsonb NOT NULL,
	CONSTRAINT job_perms_pk PRIMARY KEY (job_id)
);