CREATE TABLE resume_job (
    id            uuid          NOT NULL,
    job           uuid          NOT NULL,
    flow          uuid          NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    value         JSONB         NOT NULL DEFAULT 'null'::jsonb
                                CHECK (length(value::text) < 10 * 1024),
    PRIMARY KEY (id),
    FOREIGN KEY (flow)  REFERENCES queue(id) ON DELETE CASCADE
);

GRANT ALL ON resume_job TO windmill_user;
GRANT ALL ON resume_job TO windmill_admin;

ALTER TABLE queue
 ADD COLUMN suspend        INTEGER      NOT NULL  DEFAULT 0,
 ADD COLUMN suspend_until  TIMESTAMPTZ;
