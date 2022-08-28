CREATE TABLE endpoint
(
    workspace_id  VARCHAR(50)              NOT NULL REFERENCES workspace (id),
    path_prefix   varchar(255)             NOT NULL,
    summary       TEXT                     NOT NULL,
    description   TEXT                     NOT NULL,
    target_script varchar(255)             NOT NULL,
    created_by    VARCHAR(50)              NOT NULL,
    created_at    TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    archived      BOOLEAN                  NOT NULL DEFAULT false,
    deleted       BOOLEAN                  NOT NULL DEFAULT false,
    PRIMARY KEY (workspace_id, path_prefix)
);
