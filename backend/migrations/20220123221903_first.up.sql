-- Add migration script here
create SCHEMA IF NOT exists extensions;
create extension if not exists "uuid-ossp"      with schema extensions;


CREATE TABLE workspace (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    owner VARCHAR(50) NOT NULL,
    domain VARCHAR(30),
    deleted BOOLEAN NOT NULL DEFAULT false,
    CONSTRAINT proper_id CHECK (id ~ '^\w+(-\w+)*$')
);

INSERT INTO workspace(id, name, owner) VALUES
	('starter', 'Starter', 'admin@windmill.dev'),
	('demo', 'Demo', 'admin@windmill.dev');

CREATE TABLE script (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
	hash BIGINT NOT NULL,
	path varchar(255) NOT NULL,
	parent_hashes BIGINT[],
	summary TEXT NOT NULL,
	description TEXT NOT NULL,
    content TEXT NOT NULL,
	created_by VARCHAR(50) NOT NULL,
	created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
	archived BOOLEAN NOT NULL DEFAULT false,
	schema JSONB,
	deleted BOOLEAN NOT NULL DEFAULT false,
	is_template boolean DEFAULT false,
    PRIMARY KEY (workspace_id, hash),
    CONSTRAINT proper_id CHECK (path ~ '^[ug](\/[\w-]+){2,}$')
);

CREATE TABLE flow (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
	path varchar(255) NOT NULL,
	summary TEXT NOT NULL,
	description TEXT NOT NULL,
    value JSONB NOT NULL,
	edited_by VARCHAR(50) NOT NULL,
	edited_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
	archived BOOLEAN NOT NULL DEFAULT false,
	schema JSONB,
    PRIMARY KEY (workspace_id, path),
    CONSTRAINT proper_id CHECK (path ~ '^[ug](\/[\w-]+){2,}$')
);

INSERT INTO script(workspace_id, created_by, content, schema, summary, description, path, hash) VALUES (
'starter',
'system', 'import wmill
import psycopg2

client = wmill.Client()

def main():
    # query that returns rows will return them as a list
    res1 = query_pg("SELECT * from demo", "g/all/demodb")
                                                                                  
    # query that does not return rows will return None
    res2 = query_pg("UPDATE demo SET value = ''value''", "g/all/demodb")
                                                                                  
    # one can use RETURNING to still fetch the updated rows
    res3 = query_pg("UPDATE demo SET value = ''value'' RETURNING *", "g/all/demodb")
                                                                                  
    # output expects a dict
    return {"res1": res1, "res2": res2, "res3": res3}



def query_pg(query: str, resource: str):
    pg_con = client.get_resource(resource)
    conn = psycopg2.connect(**pg_con)
    cur = conn.cursor()
    cur.execute(f"{query};")
    if cur.description:
        return cur.fetchall()
    else:
        return None',
'{
	"$schema": "https://json-schema.org/draft/2020-12/schema",
	"properties": {},
	"required": [],
	"type": "object"
}',
'Query the demodb resource','
An example of how to use resources from scripts. In this example, we will query the demo database demodb that is set up by default on Windmill.',
'u/bot/postgres_example', 43), 
(
'starter',
'system', 
'import os

def main(name: str = "Nicolas Bourbaki"):
	print(f"Hello World and a warm welcome especially to {name}")
	print("The env variable at `g/all/pretty_secret`: ", os.environ.get("G_ALL_PRETTY_SECRET"))
	return {"len": len(name), "splitted": name.split() }',
'{
	"$schema": "https://json-schema.org/draft/2020-12/schema",
	"properties": {
		"name": {
			"description": "",
			"type": "string",
			"default": "Nicolas Bourbaki"
		}
	},
	"required": [],
	"type": "object"
}',
'Hello World',  '', 'u/bot/hello_world',  44);


CREATE INDEX index_script_on_path_created_at ON script (path, created_at);

CREATE TYPE JOB_KIND AS ENUM ('script', 'preview', 'flow', 'dependencies');

CREATE TABLE queue (
  id UUID PRIMARY KEY,
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
  parent_job UUID,
  created_by VARCHAR(50) NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
  started_at TIMESTAMP WITH TIME ZONE,
  scheduled_for TIMESTAMP WITH TIME ZONE NOT NULL,
  running BOOLEAN NOT NULL DEFAULT FALSE,
  script_hash BIGINT,
  script_path VARCHAR(255),
  args JSONB,
  logs TEXT,
  raw_code TEXT,
  canceled boolean NOT NULL DEFAULT false,
  canceled_by VARCHAR(50),
  canceled_reason TEXT,
  last_ping TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
  job_kind JOB_KIND NOT NULL DEFAULT 'script',
  env_id INTEGER,
  schedule_path VARCHAR(255),
  permissioned_as VARCHAR(55) NOT NULL DEFAULT 'g/all',
  flow_status JSONB 
);

CREATE INDEX index_queue_on_workspace_id ON queue (workspace_id);
CREATE INDEX index_queue_on_scheduled_for ON queue (scheduled_for);
CREATE INDEX index_queue_on_running ON queue (running);
CREATE INDEX index_queue_on_created ON queue (created_at);
CREATE INDEX index_queue_on_script_path ON queue (script_path);
CREATE INDEX index_queue_on_script_hash ON queue (script_hash);

CREATE TABLE completed_job (
  id UUID PRIMARY KEY,
  workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
  parent_job UUID,
  created_by VARCHAR(50) NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL,
  duration INT NOT NULL,
  success BOOLEAN NOT NULL,
  script_hash BIGINT,
  script_path VARCHAR(255),
  args JSONB,
  result JSONB,
  logs TEXT,
  deleted BOOLEAN NOT NULL DEFAULT false,
  raw_code TEXT,
  canceled boolean NOT NULL DEFAULT false,
  canceled_by VARCHAR(50),
  canceled_reason TEXT,
  job_kind JOB_KIND NOT NULL DEFAULT 'script',
  env_id INTEGER NOT NULL DEFAULT 0,
  schedule_path varchar(255),
  permissioned_as VARCHAR(55) NOT NULL DEFAULT 'g/all',
  flow_status JSONB
);

CREATE INDEX index_completed_on_workspace_id ON completed_job (workspace_id);
CREATE INDEX index_completed_on_created ON completed_job (created_at);
CREATE INDEX index_completed_on_script_path ON completed_job (script_path);
CREATE INDEX index_completed_on_script_hash ON completed_job (script_hash);

CREATE TABLE usr (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    username VARCHAR(50) NOT NULL,
    email VARCHAR(50) NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    operator BOOLEAN NOT NULL DEFAULT false,
    disabled BOOLEAN NOT NULL DEFAULT false,
    role VARCHAR(50),
    PRIMARY KEY (workspace_id, username),
	CONSTRAINT proper_username CHECK (username ~ '^[\w-]+$'),
	CONSTRAINT proper_email CHECK (email ~ '^(?:[a-z0-9!#$%&''*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&''*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$')
);

CREATE INDEX index_usr_email ON usr (email);

CREATE TYPE LOGIN_TYPE AS ENUM ('password', 'github');

CREATE TABLE password (
    email VARCHAR(50) PRIMARY KEY,
    password_hash VARCHAR(100),
    login_type LOGIN_TYPE NOT NULL,
    super_admin BOOLEAN NOT NULL DEFAULT FALSE,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    name VARCHAR(30),
    company VARCHAR(30)
);


CREATE TABLE workspace_settings (
    workspace_id VARCHAR(50) PRIMARY KEY REFERENCES workspace(id),
    slack_team_id VARCHAR(50) UNIQUE,
    slack_name VARCHAR(50),
    slack_command_script VARCHAR(255)
);


INSERT INTO workspace_settings (workspace_id) VALUES
	('starter'),
	('demo');


CREATE TABLE workspace_invite (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    email VARCHAR(50),
    is_admin bool NOT NULL DEFAULT false,
	CONSTRAINT proper_email CHECK (email ~ '^(?:[a-z0-9!#$%&''*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&''*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$'),
    PRIMARY KEY (workspace_id, email)
);

CREATE TABLE magic_link (
    email VARCHAR(50) NOT NULL,
    token VARCHAR(100) NOT NULL,
    expiration TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (NOW() + interval '1 day'),
    PRIMARY KEY (email, token)
);

CREATE TABLE token (
    token VARCHAR(50) PRIMARY KEY,
    label VARCHAR(50), 
    expiration TIMESTAMP WITH TIME ZONE,
    workspace_id VARCHAR(50) REFERENCES workspace(id),
	owner VARCHAR(55),
	email VARCHAR(50),
    super_admin BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX index_magic_link_exp ON magic_link (expiration);
CREATE INDEX index_token_exp ON token (expiration);

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('starter', 'admin@windmill.dev', 'admin', true, 'Admin'),
	('demo', 'admin@windmill.dev', 'admin', true, 'Ruben');

INSERT INTO password(email, verified, password_hash, login_type, super_admin, name, company) VALUES
	('admin@windmill.dev', true, '$argon2id$v=19$m=4096,t=3,p=1$z0Kg3qyaS14e+YHeihkJLQ$N69flI6yQ/U98pjAHtbNxbdz2f4PrJEi9Tx1VoYk1as', 'password', true, 'Admin', 'Windmill'), 
	('ruben@windmill.dev', true, '$argon2id$v=19$m=4096,t=3,p=1$z0Kg3qyaS14e+YHeihkJLQ$N69flI6yQ/U98pjAHtbNxbdz2f4PrJEi9Tx1VoYk1as', 'password', true, 'Ruben', 'Windmill'),
	('user@windmill.dev', true, '$argon2id$v=19$m=4096,t=3,p=1$z0Kg3qyaS14e+YHeihkJLQ$N69flI6yQ/U98pjAHtbNxbdz2f4PrJEi9Tx1VoYk1as', 'password', false, 'User', 'Windmill');

INSERT INTO workspace_invite(workspace_id, email, is_admin) VALUES
	('demo', 'ruben@windmill.dev', true);

CREATE TABLE variable (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    path VARCHAR(255),
    value VARCHAR(4012) NOT NULL,
    is_secret BOOLEAN NOT NULL DEFAULT FALSE,
    description VARCHAR(255) NOT NULL DEFAULT '',
    PRIMARY KEY (workspace_id, path),
    CONSTRAINT proper_id CHECK (path ~ '^[ug](\/[\w-]+){2,}$')
);

CREATE TYPE ACTION_KIND AS ENUM ('create', 'update', 'delete', 'execute');

CREATE TABLE audit (
    workspace_id VARCHAR(50) NOT NULL,
    id SERIAL,
	timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    username VARCHAR(50) NOT NULL,
    operation VARCHAR(50) NOT NULL,
    action_kind ACTION_KIND NOT NULL,
    resource VARCHAR(255),
    parameters JSONB,
    PRIMARY KEY (workspace_id, id)
);

CREATE TABLE resource_type (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    name VARCHAR(50),
    schema JSONB,
    description TEXT,
    PRIMARY KEY (workspace_id, name),
	CONSTRAINT proper_name CHECK (name ~ '^[\w-]+$')
);

CREATE TABLE resource (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    path VARCHAR(255),
    value JSONB,
    description TEXT,
    resource_type VARCHAR(50) NOT NULL,
    PRIMARY KEY (workspace_id, path),
    CONSTRAINT proper_id CHECK (path ~ '^[ug](\/[\w-]+){2,}$')
);

INSERT INTO resource_type(workspace_id, name, schema, description) VALUES
	('starter', 'postgres', '{
	"$schema": "https://json-schema.org/draft/2020-12/schema",
	"type": "object",
	"properties": {
		"dbname": {
			"description": "The database name",
			"type": "string"
		},
		"user": {
			"description": "The postgres username",
			"type": "string"
		},
		"password": {
			"description": "The postgres users password",
			"type": "string"
		},
		"sslmode": {
			"description": "The sslmode",
			"type": "string",
            "enum": ["disable", "allow", "prefer", "require", "verify-ca", "verify-full"]
		},
		"host": {
			"description": "The instance host",
			"type": "string"
		},
		"port": {
			"description": "The instance port",
			"type": "integer"
		}
	},
	"required": ["dbname", "user", "password"]
}', 'A postgres database connection resource')
	;

INSERT INTO resource(workspace_id, path, value, description, resource_type) VALUES
    ('starter', 'g/all/demodb', '{"host": "demodb.service.consul", "dbname": "demodb", 
    "user": "postgres", "password": "demodb", "sslmode": "disable", "port":"6543"}', 'demodb', 'postgres')
;


CREATE TABLE pipenv (
    id SERIAL PRIMARY KEY,
	timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    created_by VARCHAR(50) NOT NULL,
    python_version VARCHAR(20),
    dependencies VARCHAR(255)[] NOT NULL DEFAULT array[]::varchar[],
    pipfile_lock TEXT,
	job_id UUID
);

CREATE TABLE schedule(
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    path varchar(255),
    edited_by varchar(255) NOT NULL,
	edited_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    schedule VARCHAR(255) NOT NULL,
    offset_ INTEGER NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    script_path varchar(255),
    script_hash BIGINT,
    args JSONB,
    PRIMARY KEY (workspace_id, path),
	CONSTRAINT proper_id CHECK (path ~ '^[ug](\/[\w-]+){2,}$')
);

CREATE TABLE group_ (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    name VARCHAR(50),
    summary TEXT,
    PRIMARY KEY (workspace_id, name),
	CONSTRAINT proper_name CHECK (name ~ '^[\w-]+$')
);

CREATE TABLE usr_to_group(
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    group_ VARCHAR(50) NOT NULL,
	usr VARCHAR(50) NOT NULL DEFAULT 'ruben',
    CONSTRAINT fk_group FOREIGN KEY(workspace_id, group_) REFERENCES group_(workspace_id, name),
    PRIMARY KEY (workspace_id, usr, group_)
);

INSERT INTO group_ SELECT id, 'all', 'The group that always contains all users of this workspace' FROM workspace;

CREATE TABLE worker_ping(
    worker VARCHAR(50) PRIMARY KEY,
    worker_instance VARCHAR(50) NOT NULL,
    ping_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    env_id INTEGER NOT NULL DEFAULT -1,
    ip VARCHAR(50) NOT NULL DEFAULT 'NO IP',
    jobs_executed INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX worker_ping_on_ping_at ON worker_ping (ping_at);

ALTER TABLE audit ENABLE ROW LEVEL SECURITY;
CREATE POLICY audit_log_see_own ON audit FOR SELECT
USING(audit.username = current_setting('session.user') or current_setting('session.is_admin')::boolean);


INSERT INTO usr_to_group
SELECT workspace_id, 'all', username FROM (SELECT workspace_id, username from usr) as usernames
;

DROP POLICY audit_log_see_own on audit;
CREATE POLICY see_own ON audit FOR ALL
USING (audit.username = current_setting('session.user'));


ALTER TABLE queue ENABLE ROW LEVEL SECURITY;

CREATE POLICY see_own ON queue FOR ALL
USING (SPLIT_PART(queue.permissioned_as, '/', 1) = 'u' AND SPLIT_PART(queue.permissioned_as, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON queue FOR ALL
USING (SPLIT_PART(queue.permissioned_as, '/', 1) = 'g' AND SPLIT_PART(queue.permissioned_as, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

ALTER TABLE completed_job ENABLE ROW LEVEL SECURITY;


CREATE POLICY see_starter ON completed_job FOR SELECT
USING (completed_job.workspace_id = 'starter'); 

CREATE POLICY see_own ON completed_job FOR ALL
USING (SPLIT_PART(completed_job.permissioned_as, '/', 1) = 'u' AND SPLIT_PART(completed_job.permissioned_as, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON completed_job FOR ALL
USING (SPLIT_PART(completed_job.permissioned_as, '/', 1) = 'g' AND SPLIT_PART(completed_job.permissioned_as, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

CREATE POLICY schedule ON audit FOR INSERT
WITH CHECK (audit.username LIKE 'schedule-%');


DO
$do$
  DECLARE
    i text;
    arr text[] := array['resource', 'script', 'variable', 'schedule', 'flow'];
  BEGIN
  FOREACH i IN ARRAY arr
  LOOP
    EXECUTE FORMAT(
      $$

        ALTER TABLE %1$I ENABLE ROW LEVEL SECURITY;

        CREATE POLICY see_starter ON %1$I FOR SELECT
        USING (%1$I.workspace_id = 'starter');

        CREATE POLICY see_own ON %1$I FOR ALL
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'u' AND SPLIT_PART(%1$I.path, '/', 2) = current_setting('session.user'));

        CREATE POLICY see_member ON %1$I FOR ALL
        USING (SPLIT_PART(%1$I.path, '/', 1) = 'g' AND SPLIT_PART(%1$I.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));

        ALTER TABLE %1$I
        ADD COLUMN extra_perms JSONB NOT NULL DEFAULT '{}';

        CREATE INDEX %1$I_extra_perms ON %1$I USING GIN (extra_perms);

        CREATE POLICY see_extra_perms_user ON %1$I FOR ALL
        USING (extra_perms ? CONCAT('u/', current_setting('session.user')))
        WITH CHECK ((extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean);

        CREATE POLICY see_extra_perms_groups ON %1$I FOR ALL
        USING (extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
        WITH CHECK (exists(
            SELECT key, value FROM jsonb_each_text(extra_perms) 
            WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
            AND value::boolean));
      $$,
      i
    );
  END LOOP;
  END
$do$;

ALTER TABLE group_
ADD COLUMN extra_perms JSONB NOT NULL DEFAULT '{}';

CREATE INDEX group_extra_perms ON group_ USING GIN (extra_perms);

ALTER TABLE usr_to_group ENABLE ROW LEVEL SECURITY;

CREATE POLICY see_extra_perms_user ON usr_to_group FOR ALL
USING (true)
WITH CHECK (EXISTS(SELECT 1 FROM group_ WHERE usr_to_group.group_ = group_.name AND usr_to_group.workspace_id = group_.workspace_id  AND (group_.extra_perms ->> CONCAT('u/', current_setting('session.user')))::boolean));


CREATE POLICY see_extra_perms_groups ON usr_to_group FOR ALL
USING (true)
WITH CHECK (exists(
    SELECT f.* FROM group_ g, jsonb_each_text(g.extra_perms) f 
    WHERE usr_to_group.group_ = g.name AND usr_to_group.workspace_id = g.workspace_id AND SPLIT_PART(key, '/', 1) = 'g' AND key = ANY(regexp_split_to_array(current_setting('session.pgroups'), ',')::text[])
    AND value::boolean));

DO
$do$
BEGIN
    IF EXISTS (
        select usesuper from pg_user where usename = CURRENT_USER AND usesuper = 't') 
    AND NOT EXISTS (
        SELECT
        FROM   pg_catalog.pg_roles
        WHERE  rolname = 'windmill_user') THEN

        LOCK TABLE pg_catalog.pg_roles;

        CREATE ROLE windmill_user;

        GRANT ALL
        ON ALL TABLES IN SCHEMA public 
        TO windmill_user;

        GRANT ALL PRIVILEGES 
        ON ALL SEQUENCES IN SCHEMA public 
        TO windmill_user;

        ALTER DEFAULT PRIVILEGES 
            IN SCHEMA public
            GRANT ALL ON TABLES TO windmill_user;

        ALTER DEFAULT PRIVILEGES 
            IN SCHEMA public
            GRANT ALL ON SEQUENCES TO windmill_user;

    END IF;
END
$do$;

DO
$do$
BEGIN
    IF EXISTS (select usesuper from pg_user where usename = CURRENT_USER AND usesuper = 't')
    AND NOT EXISTS (
        SELECT
        FROM   pg_catalog.pg_roles
        WHERE  rolname = 'windmill_admin') THEN
        CREATE ROLE windmill_admin WITH BYPASSRLS;

        GRANT ALL
        ON ALL TABLES IN SCHEMA public 
        TO windmill_admin;

        GRANT ALL PRIVILEGES 
        ON ALL SEQUENCES IN SCHEMA public 
        TO windmill_admin;

        ALTER DEFAULT PRIVILEGES 
            IN SCHEMA public
            GRANT ALL ON TABLES TO windmill_admin;

        ALTER DEFAULT PRIVILEGES 
            IN SCHEMA public
            GRANT ALL ON SEQUENCES TO windmill_admin;
    END IF;
END
$do$;
