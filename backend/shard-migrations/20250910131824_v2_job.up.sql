-- Add up migration script here

--
-- Name: v2_job; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TYPE JOB_KIND AS ENUM (
    'script',
    'script_hub',
    'preview',
    'dependencies',
    'flow',
    'flowpreview',
    'singlescriptflow',
    'identity',
    'flowdependencies',
    'appdependencies',
    'noop',
    'deploymentcallback',
    'flowscript',
    'flownode',
    'appscript',
    'aiagent'
);

CREATE TYPE SCRIPT_LANG AS ENUM (
    'nativets',
    'deno',
    'python3',
    'go',
    'bash',
    'powershell',
    'postgresql',
    'bun',
    'bunnative',
    'mysql',
    'bigquery',
    'snowflake',
    'graphql',
    'mssql',
    'oracledb',
    'duckdb',
    'php',
    'rust',
    'ansible',
    'csharp',
    'nu',
    'java',
    'ruby'
);

CREATE TYPE JOB_TRIGGER_KIND AS ENUM (
    'webhook',
    'http',
    'websocket',
    'kafka',
    'email',
    'nats',
    'mqtt',
    'sqs',
    'postgres',
    'schedule',
    'gcp'
);




CREATE TABLE public.v2_job (
    id uuid NOT NULL,
    raw_code text,
    raw_lock text,
    raw_flow jsonb,
    tag character varying(50) DEFAULT 'other'::character varying NOT NULL,
    workspace_id character varying(50) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by character varying(255) DEFAULT 'missing'::character varying NOT NULL,
    permissioned_as character varying(55) DEFAULT 'g/all'::character varying NOT NULL,
    permissioned_as_email character varying(255) DEFAULT 'missing@email.xyz'::character varying NOT NULL,
    kind public.job_kind DEFAULT 'script'::public.job_kind NOT NULL,
    runnable_id bigint,
    runnable_path character varying(255),
    parent_job uuid,
    root_job uuid,
    script_lang public.script_lang DEFAULT 'python3'::public.script_lang,
    script_entrypoint_override character varying(255),
    flow_step integer,
    flow_step_id character varying(255),
    flow_innermost_root_job uuid,
    trigger character varying(255),
    trigger_kind public.job_trigger_kind,
    same_worker boolean DEFAULT false NOT NULL,
    visible_to_owner boolean DEFAULT true NOT NULL,
    concurrent_limit integer,
    concurrency_time_window_s integer,
    cache_ttl integer,
    timeout integer,
    priority smallint,
    preprocessed boolean,
    args jsonb,
    labels text[],
    pre_run_error text
);


-- ALTER TABLE public.v2_job OWNER TO postgres;

--
-- Name: v2_job job_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job
    ADD CONSTRAINT job_pkey PRIMARY KEY (id);


--
-- Name: ix_job_created_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_job_created_at ON public.v2_job USING btree (created_at DESC);


--
-- Name: ix_job_root_job_index_by_path_2; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_job_root_job_index_by_path_2 ON public.v2_job USING btree (workspace_id, runnable_path, created_at DESC) WHERE (parent_job IS NULL);


--
-- Name: ix_job_workspace_id_created_at_new_3; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_job_workspace_id_created_at_new_3 ON public.v2_job USING btree (workspace_id, created_at DESC);


--
-- Name: ix_job_workspace_id_created_at_new_5; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_job_workspace_id_created_at_new_5 ON public.v2_job USING btree (workspace_id, created_at DESC) WHERE ((kind = ANY (ARRAY['preview'::public.job_kind, 'flowpreview'::public.job_kind])) AND (parent_job IS NULL));


--
-- Name: ix_job_workspace_id_created_at_new_8; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_job_workspace_id_created_at_new_8 ON public.v2_job USING btree (workspace_id, created_at DESC) WHERE ((kind = 'deploymentcallback'::public.job_kind) AND (parent_job IS NULL));


--
-- Name: ix_job_workspace_id_created_at_new_9; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_job_workspace_id_created_at_new_9 ON public.v2_job USING btree (workspace_id, created_at DESC) WHERE ((kind = ANY (ARRAY['dependencies'::public.job_kind, 'flowdependencies'::public.job_kind, 'appdependencies'::public.job_kind])) AND (parent_job IS NULL));


--
-- Name: ix_v2_job_labels; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_v2_job_labels ON public.v2_job USING gin (labels) WHERE (labels IS NOT NULL);


--
-- Name: ix_v2_job_workspace_id_created_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_v2_job_workspace_id_created_at ON public.v2_job USING btree (workspace_id, created_at DESC) WHERE ((kind = ANY (ARRAY['script'::public.job_kind, 'flow'::public.job_kind, 'singlescriptflow'::public.job_kind])) AND (parent_job IS NULL));


--
-- Name: v2_job admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

-- CREATE POLICY admin_policy ON public.v2_job TO windmill_admin USING (true);


--
-- Name: v2_job see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

-- CREATE POLICY see_folder_extra_perms_user ON public.v2_job TO windmill_user USING (((visible_to_owner IS TRUE) AND (split_part((runnable_path)::text, '/'::text, 1) = 'f'::text) AND (split_part((runnable_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: v2_job see_member; Type: POLICY; Schema: public; Owner: postgres
--
 
-- CREATE POLICY see_member ON public.v2_job TO windmill_user USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'g'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: v2_job see_member_path; Type: POLICY; Schema: public; Owner: postgres
--

-- CREATE POLICY see_member_path ON public.v2_job TO windmill_user USING (((visible_to_owner IS TRUE) AND (split_part((runnable_path)::text, '/'::text, 1) = 'g'::text) AND (split_part((runnable_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: v2_job see_own; Type: POLICY; Schema: public; Owner: postgres
--

-- CREATE POLICY see_own ON public.v2_job TO windmill_user USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'u'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: v2_job see_own_path; Type: POLICY; Schema: public; Owner: postgres
--

-- CREATE POLICY see_own_path ON public.v2_job TO windmill_user USING (((visible_to_owner IS TRUE) AND (split_part((runnable_path)::text, '/'::text, 1) = 'u'::text) AND (split_part((runnable_path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: v2_job; Type: ROW SECURITY; Schema: public; Owner: postgres
--

-- ALTER TABLE public.v2_job ENABLE ROW LEVEL SECURITY;

--
-- Name: TABLE v2_job; Type: ACL; Schema: public; Owner: postgres
--

-- GRANT ALL ON TABLE public.v2_job TO windmill_user;
-- GRANT ALL ON TABLE public.v2_job TO windmill_admin;