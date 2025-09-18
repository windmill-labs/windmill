CREATE TABLE public.v2_job_queue (
    id uuid NOT NULL,
    workspace_id character varying(50) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    started_at timestamp with time zone,
    scheduled_for timestamp with time zone NOT NULL,
    running boolean DEFAULT false NOT NULL,
    canceled_by character varying(255),
    canceled_reason text,
    suspend integer DEFAULT 0 NOT NULL,
    suspend_until timestamp with time zone,
    tag character varying(255) DEFAULT 'other'::character varying NOT NULL,
    priority smallint,
    worker character varying(255),
    extras jsonb
);

ALTER TABLE ONLY public.v2_job_queue
    ADD CONSTRAINT queue_pkey PRIMARY KEY (id);

CREATE INDEX queue_sort_v2 
    ON public.v2_job_queue (priority DESC NULLS LAST, scheduled_for, tag) 
    WHERE (running = false);

CREATE INDEX queue_suspended 
    ON public.v2_job_queue (priority DESC NULLS LAST, created_at, suspend_until, suspend, tag) 
    WHERE (suspend_until IS NOT NULL);

CREATE INDEX root_queue_index_by_path 
    ON public.v2_job_queue (workspace_id, created_at);

CREATE INDEX v2_job_queue_suspend 
    ON public.v2_job_queue (workspace_id, suspend) 
    WHERE (suspend > 0);

---CREATE POLICY admin_policy ON public.v2_job_queue 
    ---TO windmill_admin USING (true);

---GRANT SELECT, INSERT, REFERENCES, DELETE, TRIGGER, TRUNCATE, UPDATE 
    ---ON public.v2_job_queue TO windmill_user;

---GRANT SELECT, INSERT, REFERENCES, DELETE, TRIGGER, TRUNCATE, UPDATE 
    ---ON public.v2_job_queue TO windmill_admin;
