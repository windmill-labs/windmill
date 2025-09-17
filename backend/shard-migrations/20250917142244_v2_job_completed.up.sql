-- Add up migration script here

CREATE TABLE public.v2_job_completed (
    id uuid NOT NULL,
    workspace_id character varying(50) NOT NULL,
    duration_ms bigint NOT NULL,
    result jsonb,
    deleted boolean DEFAULT false NOT NULL,
    canceled_by character varying(50),
    canceled_reason text,
    flow_status jsonb,
    started_at timestamp with time zone DEFAULT now(),
    memory_peak integer,
    status public.job_status NOT NULL,
    completed_at timestamp with time zone DEFAULT now() NOT NULL,
    worker character varying(255),
    workflow_as_code_status jsonb,
    result_columns text[],
    retries uuid[],
    extras jsonb
);


-- ALTER TABLE public.v2_job_completed OWNER TO postgres;

--
-- Name: v2_job_completed completed_job_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job_completed
    ADD CONSTRAINT completed_job_pkey PRIMARY KEY (id);


--
-- Name: ix_completed_job_workspace_id_started_at_new_2; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_completed_job_workspace_id_started_at_new_2 ON public.v2_job_completed USING btree (workspace_id, started_at DESC);


--
-- Name: ix_job_completed_completed_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_job_completed_completed_at ON public.v2_job_completed USING btree (completed_at DESC);


--
-- Name: labeled_jobs_on_jobs; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX labeled_jobs_on_jobs ON public.v2_job_completed USING gin (((result -> 'wm_labels'::text))) WHERE (result ? 'wm_labels'::text);


--
-- Name: v2_job_completed admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

-- CREATE POLICY admin_policy ON public.v2_job_completed TO windmill_admin USING (true);


--
-- Name: TABLE v2_job_completed; Type: ACL; Schema: public; Owner: postgres
--

-- GRANT ALL ON TABLE public.v2_job_completed TO windmill_user;
-- GRANT ALL ON TABLE public.v2_job_completed TO windmill_admin;




