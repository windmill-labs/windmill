-- Add up migration script here
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'appscript';

-- Same as `app_version` but with a "lite" value (w/ `inlineScript.{code,lock}`).
CREATE TABLE app_version_lite (
    id BIGSERIAL PRIMARY KEY,
    value JSONB,
    FOREIGN KEY (id) REFERENCES app_version (id) ON DELETE CASCADE
);

GRANT ALL ON app_version_lite TO windmill_user;
GRANT ALL ON app_version_lite TO windmill_admin;

-- App `inlineScript`.
CREATE TABLE app_script (
    id BIGSERIAL PRIMARY KEY,
    app BIGSERIAL NOT NULL,
    hash CHAR(64) NOT NULL UNIQUE, -- sha256 of `app`, `lock`, `code`.
    lock TEXT,
    code TEXT NOT NULL,
    code_sha256 CHAR(64) NOT NULL, -- used to retrieve the policy.
    FOREIGN KEY (app) REFERENCES app (id) ON DELETE CASCADE
);

GRANT ALL ON app_script TO windmill_user;
GRANT ALL ON app_script TO windmill_admin;
