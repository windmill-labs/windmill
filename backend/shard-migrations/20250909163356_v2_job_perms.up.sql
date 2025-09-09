-- Add up migration script here

CREATE TABLE public.job_perms (
    job_id uuid NOT NULL,
    email character varying(255) NOT NULL,
    username character varying(50) NOT NULL,
    is_admin boolean NOT NULL,
    is_operator boolean NOT NULL,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    workspace_id character varying(50) NOT NULL,
    groups text[] NOT NULL,
    folders jsonb[] NOT NULL
);


ALTER TABLE public.job_perms OWNER TO postgres;


ALTER TABLE ONLY public.job_perms
    ADD CONSTRAINT job_perms_pk PRIMARY KEY (job_id);


