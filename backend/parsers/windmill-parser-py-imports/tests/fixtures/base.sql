--
-- PostgreSQL database dump
--

-- Dumped from database version 15.3
-- Dumped by pg_dump version 15.3

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: _sqlx_test; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA _sqlx_test;


ALTER SCHEMA _sqlx_test OWNER TO postgres;

--
-- Name: extensions; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA extensions;


ALTER SCHEMA extensions OWNER TO postgres;

--
-- Name: uuid-ossp; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA extensions;


--
-- Name: EXTENSION "uuid-ossp"; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION "uuid-ossp" IS 'generate universally unique identifiers (UUIDs)';


--
-- Name: action_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.action_kind AS ENUM (
    'create',
    'update',
    'delete',
    'execute'
);


ALTER TYPE public.action_kind OWNER TO postgres;

--
-- Name: draft_type; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.draft_type AS ENUM (
    'script',
    'flow',
    'app'
);


ALTER TYPE public.draft_type OWNER TO postgres;

--
-- Name: favorite_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.favorite_kind AS ENUM (
    'app',
    'script',
    'flow',
    'raw_app'
);


ALTER TYPE public.favorite_kind OWNER TO postgres;

--
-- Name: job_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.job_kind AS ENUM (
    'script',
    'preview',
    'flow',
    'dependencies',
    'flowpreview',
    'script_hub',
    'identity',
    'flowdependencies',
    'http',
    'graphql',
    'postgresql',
    'noop'
);


ALTER TYPE public.job_kind OWNER TO postgres;

--
-- Name: login_type; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.login_type AS ENUM (
    'password',
    'github'
);


ALTER TYPE public.login_type OWNER TO postgres;

--
-- Name: runnable_type; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.runnable_type AS ENUM (
    'ScriptHash',
    'ScriptPath',
    'FlowPath'
);


ALTER TYPE public.runnable_type OWNER TO postgres;

--
-- Name: script_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.script_kind AS ENUM (
    'script',
    'trigger',
    'failure',
    'command',
    'approval'
);


ALTER TYPE public.script_kind OWNER TO postgres;

--
-- Name: script_lang; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.script_lang AS ENUM (
    'python3',
    'deno',
    'go',
    'bash',
    'postgresql',
    'nativets',
    'Nativets',
    'bun',
    'mysql'
);


ALTER TYPE public.script_lang OWNER TO postgres;

--
-- Name: workspace_key_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.workspace_key_kind AS ENUM (
    'cloud'
);


ALTER TYPE public.workspace_key_kind OWNER TO postgres;

--
-- Name: database_ids; Type: SEQUENCE; Schema: _sqlx_test; Owner: postgres
--

CREATE SEQUENCE _sqlx_test.database_ids
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE _sqlx_test.database_ids OWNER TO postgres;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: databases; Type: TABLE; Schema: _sqlx_test; Owner: postgres
--

CREATE TABLE _sqlx_test.databases (
    db_name text NOT NULL,
    test_path text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE _sqlx_test.databases OWNER TO postgres;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO postgres;

--
-- Name: account; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.account (
    workspace_id character varying(50) NOT NULL,
    id integer NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    refresh_token character varying(1500) NOT NULL,
    owner character varying(50) NOT NULL,
    client character varying(50) NOT NULL,
    refresh_error text
);


ALTER TABLE public.account OWNER TO postgres;

--
-- Name: account_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.account_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.account_id_seq OWNER TO postgres;

--
-- Name: account_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.account_id_seq OWNED BY public.account.id;


--
-- Name: app; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.app (
    id bigint NOT NULL,
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    summary character varying(1000) DEFAULT ''::character varying NOT NULL,
    policy jsonb NOT NULL,
    versions bigint[] NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    draft_only boolean
);


ALTER TABLE public.app OWNER TO postgres;

--
-- Name: app_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.app_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.app_id_seq OWNER TO postgres;

--
-- Name: app_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.app_id_seq OWNED BY public.app.id;


--
-- Name: app_version; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.app_version (
    id bigint NOT NULL,
    app_id bigint NOT NULL,
    value json NOT NULL,
    created_by character varying(50) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.app_version OWNER TO postgres;

--
-- Name: app_version_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.app_version_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.app_version_id_seq OWNER TO postgres;

--
-- Name: app_version_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.app_version_id_seq OWNED BY public.app_version.id;


--
-- Name: audit; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.audit (
    workspace_id character varying(50) NOT NULL,
    id integer NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    username character varying(255) NOT NULL,
    operation character varying(50) NOT NULL,
    action_kind public.action_kind NOT NULL,
    resource character varying(255),
    parameters jsonb
);


ALTER TABLE public.audit OWNER TO postgres;

--
-- Name: audit_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.audit_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.audit_id_seq OWNER TO postgres;

--
-- Name: audit_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.audit_id_seq OWNED BY public.audit.id;


--
-- Name: capture; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.capture (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by character varying(50) NOT NULL,
    payload jsonb DEFAULT 'null'::jsonb NOT NULL,
    CONSTRAINT capture_payload_too_big CHECK ((length((payload)::text) < (512 * 1024)))
);


ALTER TABLE public.capture OWNER TO postgres;

--
-- Name: completed_job; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.completed_job (
    id uuid NOT NULL,
    workspace_id character varying(50) NOT NULL,
    parent_job uuid,
    created_by character varying(255) NOT NULL,
    created_at timestamp with time zone NOT NULL,
    duration_ms integer NOT NULL,
    success boolean NOT NULL,
    script_hash bigint,
    script_path character varying(255),
    args jsonb,
    result jsonb,
    logs text,
    deleted boolean DEFAULT false NOT NULL,
    raw_code text,
    canceled boolean DEFAULT false NOT NULL,
    canceled_by character varying(50),
    canceled_reason text,
    job_kind public.job_kind DEFAULT 'script'::public.job_kind NOT NULL,
    env_id integer DEFAULT 0 NOT NULL,
    schedule_path character varying(255),
    permissioned_as character varying(55) DEFAULT 'g/all'::character varying NOT NULL,
    flow_status jsonb,
    raw_flow jsonb,
    is_flow_step boolean DEFAULT false,
    language public.script_lang DEFAULT 'python3'::public.script_lang,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    is_skipped boolean DEFAULT false NOT NULL,
    raw_lock text,
    email character varying(50) DEFAULT 'missing@email.xyz'::character varying NOT NULL,
    visible_to_owner boolean DEFAULT true,
    mem_peak integer,
    tag character varying(50) DEFAULT 'other'::character varying NOT NULL
);


ALTER TABLE public.completed_job OWNER TO postgres;

--
-- Name: demo; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.demo (
    key text,
    value bigint
);


ALTER TABLE public.demo OWNER TO postgres;

--
-- Name: draft; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.draft (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    typ public.draft_type NOT NULL,
    value json NOT NULL,
    created_at timestamp without time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.draft OWNER TO postgres;

--
-- Name: email_to_igroup; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.email_to_igroup (
    email character varying(255) NOT NULL,
    igroup character varying(255) NOT NULL
);


ALTER TABLE public.email_to_igroup OWNER TO postgres;

--
-- Name: favorite; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.favorite (
    usr character varying(50) NOT NULL,
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    favorite_kind public.favorite_kind NOT NULL
);


ALTER TABLE public.favorite OWNER TO postgres;

--
-- Name: flow; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.flow (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    summary text NOT NULL,
    description text NOT NULL,
    value jsonb NOT NULL,
    edited_by character varying(50) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    archived boolean DEFAULT false NOT NULL,
    schema json,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    dependency_job uuid,
    draft_only boolean,
    tag character varying(50),
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


ALTER TABLE public.flow OWNER TO postgres;

--
-- Name: folder; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.folder (
    name character varying(255) NOT NULL,
    workspace_id character varying(50) NOT NULL,
    display_name character varying(100) NOT NULL,
    owners character varying(255)[] NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL
);


ALTER TABLE public.folder OWNER TO postgres;

--
-- Name: group_; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.group_ (
    workspace_id character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    summary text,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    CONSTRAINT proper_name CHECK (((name)::text ~ '^[\w-]+$'::text))
);


ALTER TABLE public.group_ OWNER TO postgres;

--
-- Name: input; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.input (
    id uuid NOT NULL,
    workspace_id character varying(50) NOT NULL,
    runnable_id character varying(255) NOT NULL,
    runnable_type public.runnable_type NOT NULL,
    name text NOT NULL,
    args jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by character varying(50) NOT NULL,
    is_public boolean DEFAULT false NOT NULL
);


ALTER TABLE public.input OWNER TO postgres;

--
-- Name: instance_group; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.instance_group (
    name character varying(255) NOT NULL,
    summary character varying(2000),
    external_id character varying(1000)
);


ALTER TABLE public.instance_group OWNER TO postgres;

--
-- Name: magic_link; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.magic_link (
    email character varying(50) NOT NULL,
    token character varying(100) NOT NULL,
    expiration timestamp with time zone DEFAULT (now() + '1 day'::interval) NOT NULL
);


ALTER TABLE public.magic_link OWNER TO postgres;

--
-- Name: password; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.password (
    email character varying(50) NOT NULL,
    password_hash character varying(100),
    login_type character varying(50) NOT NULL,
    super_admin boolean DEFAULT false NOT NULL,
    verified boolean DEFAULT false NOT NULL,
    name character varying(255),
    company character varying(255),
    first_time_user boolean DEFAULT false NOT NULL
);


ALTER TABLE public.password OWNER TO postgres;

--
-- Name: pip_resolution_cache; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.pip_resolution_cache (
    hash character varying(255) NOT NULL,
    expiration timestamp without time zone NOT NULL,
    lockfile text NOT NULL
);


ALTER TABLE public.pip_resolution_cache OWNER TO postgres;

--
-- Name: queue; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.queue (
    id uuid NOT NULL,
    workspace_id character varying(50) NOT NULL,
    parent_job uuid,
    created_by character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    started_at timestamp with time zone,
    scheduled_for timestamp with time zone NOT NULL,
    running boolean DEFAULT false NOT NULL,
    script_hash bigint,
    script_path character varying(255),
    args jsonb,
    logs text,
    raw_code text,
    canceled boolean DEFAULT false NOT NULL,
    canceled_by character varying(50),
    canceled_reason text,
    last_ping timestamp with time zone DEFAULT now() NOT NULL,
    job_kind public.job_kind DEFAULT 'script'::public.job_kind NOT NULL,
    env_id integer,
    schedule_path character varying(255),
    permissioned_as character varying(55) DEFAULT 'g/all'::character varying NOT NULL,
    flow_status jsonb,
    raw_flow jsonb,
    is_flow_step boolean DEFAULT false,
    language public.script_lang DEFAULT 'python3'::public.script_lang,
    suspend integer DEFAULT 0 NOT NULL,
    suspend_until timestamp with time zone,
    same_worker boolean DEFAULT false,
    raw_lock text,
    pre_run_error text,
    email character varying(50) DEFAULT 'missing@email.xyz'::character varying NOT NULL,
    visible_to_owner boolean DEFAULT true,
    mem_peak integer,
    root_job uuid,
    leaf_jobs jsonb,
    tag character varying(50) DEFAULT 'other'::character varying NOT NULL,
    concurrent_limit integer,
    concurrency_time_window_s integer
);


ALTER TABLE public.queue OWNER TO postgres;

--
-- Name: raw_app; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.raw_app (
    path character varying(255) NOT NULL,
    version integer DEFAULT 0 NOT NULL,
    workspace_id character varying(50) NOT NULL,
    summary character varying(1000) DEFAULT ''::character varying NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    data text NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL
);


ALTER TABLE public.raw_app OWNER TO postgres;

--
-- Name: resource; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.resource (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    value jsonb,
    description text,
    resource_type character varying(50) NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


ALTER TABLE public.resource OWNER TO postgres;

--
-- Name: resource_type; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.resource_type (
    workspace_id character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    schema jsonb,
    description text,
    CONSTRAINT proper_name CHECK (((name)::text ~ '^[\w-]+$'::text))
);


ALTER TABLE public.resource_type OWNER TO postgres;

--
-- Name: resume_job; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.resume_job (
    id uuid NOT NULL,
    job uuid NOT NULL,
    flow uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    value jsonb DEFAULT 'null'::jsonb NOT NULL,
    approver character varying(50),
    resume_id integer DEFAULT 0 NOT NULL,
    CONSTRAINT resume_job_value_check CHECK ((length((value)::text) < (10 * 1024)))
);


ALTER TABLE public.resume_job OWNER TO postgres;

--
-- Name: schedule; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.schedule (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    edited_by character varying(255) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    schedule character varying(255) NOT NULL,
    enabled boolean DEFAULT true NOT NULL,
    script_path character varying(255) NOT NULL,
    args jsonb,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    is_flow boolean DEFAULT false NOT NULL,
    email character varying(50) DEFAULT 'missing@email.xyz'::character varying NOT NULL,
    error text,
    timezone character varying(255) DEFAULT 'UTC'::character varying NOT NULL,
    on_failure character varying(1000) DEFAULT NULL::character varying,
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


ALTER TABLE public.schedule OWNER TO postgres;

--
-- Name: script; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.script (
    workspace_id character varying(50) NOT NULL,
    hash bigint NOT NULL,
    path character varying(255) NOT NULL,
    parent_hashes bigint[],
    summary text NOT NULL,
    description text NOT NULL,
    content text NOT NULL,
    created_by character varying(50) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    archived boolean DEFAULT false NOT NULL,
    schema json,
    deleted boolean DEFAULT false NOT NULL,
    is_template boolean DEFAULT false,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    lock text,
    lock_error_logs text,
    language public.script_lang DEFAULT 'python3'::public.script_lang NOT NULL,
    kind public.script_kind DEFAULT 'script'::public.script_kind NOT NULL,
    tag character varying(50),
    draft_only boolean,
    envs character varying(1000)[],
    concurrent_limit integer,
    concurrency_time_window_s integer,
    dedicated_worker boolean,
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


ALTER TABLE public.script OWNER TO postgres;

--
-- Name: token; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.token (
    token character varying(50) NOT NULL,
    label character varying(50),
    expiration timestamp with time zone,
    workspace_id character varying(50),
    owner character varying(55),
    email character varying(50),
    super_admin boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone DEFAULT now() NOT NULL,
    scopes text[]
);


ALTER TABLE public.token OWNER TO postgres;

--
-- Name: usage; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.usage (
    id character varying(50) NOT NULL,
    is_workspace boolean NOT NULL,
    month_ integer NOT NULL,
    usage integer NOT NULL
);


ALTER TABLE public.usage OWNER TO postgres;

--
-- Name: usr; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.usr (
    workspace_id character varying(50) NOT NULL,
    username character varying(50) NOT NULL,
    email character varying(50) NOT NULL,
    is_admin boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    operator boolean DEFAULT false NOT NULL,
    disabled boolean DEFAULT false NOT NULL,
    role character varying(50),
    CONSTRAINT proper_email CHECK (((email)::text ~* '^(?:[a-z0-9!#$%&''*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&''*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$'::text)),
    CONSTRAINT proper_username CHECK (((username)::text ~ '^[\w-]+$'::text))
);


ALTER TABLE public.usr OWNER TO postgres;

--
-- Name: usr_to_group; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.usr_to_group (
    workspace_id character varying(50) NOT NULL,
    group_ character varying(50) NOT NULL,
    usr character varying(50) DEFAULT 'ruben'::character varying NOT NULL
);


ALTER TABLE public.usr_to_group OWNER TO postgres;

--
-- Name: variable; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.variable (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    value character varying(15000) NOT NULL,
    is_secret boolean DEFAULT false NOT NULL,
    description character varying(255) DEFAULT ''::character varying NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    account integer,
    is_oauth boolean DEFAULT false NOT NULL,
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


ALTER TABLE public.variable OWNER TO postgres;

--
-- Name: worker_ping; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.worker_ping (
    worker character varying(50) NOT NULL,
    worker_instance character varying(50) NOT NULL,
    ping_at timestamp with time zone DEFAULT now() NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    ip character varying(50) DEFAULT 'NO IP'::character varying NOT NULL,
    jobs_executed integer DEFAULT 0 NOT NULL,
    custom_tags text[]
);


ALTER TABLE public.worker_ping OWNER TO postgres;

--
-- Name: workspace; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.workspace (
    id character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    owner character varying(50) NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    premium boolean DEFAULT false NOT NULL,
    CONSTRAINT proper_id CHECK (((id)::text ~ '^\w+(-\w+)*$'::text))
);


ALTER TABLE public.workspace OWNER TO postgres;

--
-- Name: workspace_invite; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.workspace_invite (
    workspace_id character varying(50) NOT NULL,
    email character varying(50) NOT NULL,
    is_admin boolean DEFAULT false NOT NULL,
    operator boolean DEFAULT false NOT NULL,
    CONSTRAINT proper_email CHECK (((email)::text ~* '^(?:[a-z0-9!#$%&''*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&''*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$'::text))
);


ALTER TABLE public.workspace_invite OWNER TO postgres;

--
-- Name: workspace_key; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.workspace_key (
    workspace_id character varying(50) NOT NULL,
    kind public.workspace_key_kind NOT NULL,
    key character varying(255) DEFAULT 'changeme'::character varying NOT NULL
);


ALTER TABLE public.workspace_key OWNER TO postgres;

--
-- Name: workspace_settings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.workspace_settings (
    workspace_id character varying(50) NOT NULL,
    slack_team_id character varying(50),
    slack_name character varying(50),
    slack_command_script character varying(255),
    slack_email character varying(50) DEFAULT 'missing@email.xyz'::character varying NOT NULL,
    auto_invite_domain character varying(50),
    auto_invite_operator boolean DEFAULT false,
    customer_id character varying(100),
    plan character varying(40),
    webhook text,
    deploy_to character varying(255),
    error_handler character varying(1000),
    openai_resource_path character varying(1000)
);


ALTER TABLE public.workspace_settings OWNER TO postgres;

--
-- Name: account id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.account ALTER COLUMN id SET DEFAULT nextval('public.account_id_seq'::regclass);


--
-- Name: app id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app ALTER COLUMN id SET DEFAULT nextval('public.app_id_seq'::regclass);


--
-- Name: app_version id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_version ALTER COLUMN id SET DEFAULT nextval('public.app_version_id_seq'::regclass);


--
-- Name: audit id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.audit ALTER COLUMN id SET DEFAULT nextval('public.audit_id_seq'::regclass);


--
-- Name: databases databases_pkey; Type: CONSTRAINT; Schema: _sqlx_test; Owner: postgres
--

ALTER TABLE ONLY _sqlx_test.databases
    ADD CONSTRAINT databases_pkey PRIMARY KEY (db_name);


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: account account_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.account
    ADD CONSTRAINT account_pkey PRIMARY KEY (workspace_id, id);


--
-- Name: app app_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app
    ADD CONSTRAINT app_pkey PRIMARY KEY (id);


--
-- Name: app_version app_version_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_version
    ADD CONSTRAINT app_version_pkey PRIMARY KEY (id);


--
-- Name: audit audit_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.audit
    ADD CONSTRAINT audit_pkey PRIMARY KEY (workspace_id, id);


--
-- Name: capture capture_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.capture
    ADD CONSTRAINT capture_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: completed_job completed_job_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.completed_job
    ADD CONSTRAINT completed_job_pkey PRIMARY KEY (id);


--
-- Name: draft draft_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.draft
    ADD CONSTRAINT draft_pkey PRIMARY KEY (workspace_id, path, typ);


--
-- Name: email_to_igroup email_to_igroup_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.email_to_igroup
    ADD CONSTRAINT email_to_igroup_pkey PRIMARY KEY (email, igroup);


--
-- Name: favorite favorite_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.favorite
    ADD CONSTRAINT favorite_pkey PRIMARY KEY (usr, workspace_id, favorite_kind, path);


--
-- Name: flow flow_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow
    ADD CONSTRAINT flow_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: folder folder_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.folder
    ADD CONSTRAINT folder_pkey PRIMARY KEY (workspace_id, name);


--
-- Name: group_ group__pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_
    ADD CONSTRAINT group__pkey PRIMARY KEY (workspace_id, name);


--
-- Name: input input_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.input
    ADD CONSTRAINT input_pkey PRIMARY KEY (id);


--
-- Name: instance_group instance_group_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.instance_group
    ADD CONSTRAINT instance_group_pkey PRIMARY KEY (name);


--
-- Name: magic_link magic_link_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.magic_link
    ADD CONSTRAINT magic_link_pkey PRIMARY KEY (email, token);


--
-- Name: password password_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.password
    ADD CONSTRAINT password_pkey PRIMARY KEY (email);


--
-- Name: pip_resolution_cache pip_resolution_cache_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.pip_resolution_cache
    ADD CONSTRAINT pip_resolution_cache_pkey PRIMARY KEY (hash);


--
-- Name: queue queue_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.queue
    ADD CONSTRAINT queue_pkey PRIMARY KEY (id);


--
-- Name: raw_app raw_app_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.raw_app
    ADD CONSTRAINT raw_app_pkey PRIMARY KEY (path);


--
-- Name: resource resource_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.resource
    ADD CONSTRAINT resource_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: resource_type resource_type_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.resource_type
    ADD CONSTRAINT resource_type_pkey PRIMARY KEY (workspace_id, name);


--
-- Name: resume_job resume_job_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.resume_job
    ADD CONSTRAINT resume_job_pkey PRIMARY KEY (id);


--
-- Name: schedule schedule_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.schedule
    ADD CONSTRAINT schedule_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: script script_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.script
    ADD CONSTRAINT script_pkey PRIMARY KEY (workspace_id, hash);


--
-- Name: token token_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.token
    ADD CONSTRAINT token_pkey PRIMARY KEY (token);


--
-- Name: app unique_path_workspace_id; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app
    ADD CONSTRAINT unique_path_workspace_id UNIQUE (workspace_id, path);


--
-- Name: usage usage_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.usage
    ADD CONSTRAINT usage_pkey PRIMARY KEY (id, is_workspace, month_);


--
-- Name: usr usr_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.usr
    ADD CONSTRAINT usr_pkey PRIMARY KEY (workspace_id, username);


--
-- Name: usr_to_group usr_to_group_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.usr_to_group
    ADD CONSTRAINT usr_to_group_pkey PRIMARY KEY (workspace_id, usr, group_);


--
-- Name: variable variable_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.variable
    ADD CONSTRAINT variable_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: worker_ping worker_ping_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.worker_ping
    ADD CONSTRAINT worker_ping_pkey PRIMARY KEY (worker);


--
-- Name: workspace_invite workspace_invite_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_invite
    ADD CONSTRAINT workspace_invite_pkey PRIMARY KEY (workspace_id, email);


--
-- Name: workspace_key workspace_key_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_key
    ADD CONSTRAINT workspace_key_pkey PRIMARY KEY (workspace_id, kind);


--
-- Name: workspace workspace_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace
    ADD CONSTRAINT workspace_pkey PRIMARY KEY (id);


--
-- Name: workspace_settings workspace_settings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_settings
    ADD CONSTRAINT workspace_settings_pkey PRIMARY KEY (workspace_id);


--
-- Name: workspace_settings workspace_settings_slack_team_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_settings
    ADD CONSTRAINT workspace_settings_slack_team_id_key UNIQUE (slack_team_id);


--
-- Name: databases_created_at; Type: INDEX; Schema: _sqlx_test; Owner: postgres
--

CREATE INDEX databases_created_at ON _sqlx_test.databases USING btree (created_at);


--
-- Name: flow_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX flow_extra_perms ON public.flow USING gin (extra_perms);


--
-- Name: folder_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX folder_extra_perms ON public.folder USING gin (extra_perms);


--
-- Name: folder_owners; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX folder_owners ON public.folder USING gin (owners);


--
-- Name: group_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX group_extra_perms ON public.group_ USING gin (extra_perms);


--
-- Name: index_completed_on_created; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_completed_on_created ON public.completed_job USING btree (created_at);


--
-- Name: index_completed_on_script_hash; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_completed_on_script_hash ON public.completed_job USING btree (script_hash);


--
-- Name: index_completed_on_script_path; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_completed_on_script_path ON public.completed_job USING btree (script_path);


--
-- Name: index_completed_on_workspace_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_completed_on_workspace_id ON public.completed_job USING btree (workspace_id);


--
-- Name: index_magic_link_exp; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_magic_link_exp ON public.magic_link USING btree (expiration);


--
-- Name: index_queue_on_created; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_queue_on_created ON public.queue USING btree (created_at);


--
-- Name: index_queue_on_running; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_queue_on_running ON public.queue USING btree (running);


--
-- Name: index_queue_on_scheduled_for; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_queue_on_scheduled_for ON public.queue USING btree (scheduled_for);


--
-- Name: index_queue_on_script_hash; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_queue_on_script_hash ON public.queue USING btree (script_hash);


--
-- Name: index_queue_on_script_path; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_queue_on_script_path ON public.queue USING btree (script_path);


--
-- Name: index_queue_on_workspace_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_queue_on_workspace_id ON public.queue USING btree (workspace_id);


--
-- Name: index_script_on_path_created_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_script_on_path_created_at ON public.script USING btree (path, created_at);


--
-- Name: index_token_exp; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_token_exp ON public.token USING btree (expiration);


--
-- Name: index_usr_email; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_usr_email ON public.usr USING btree (email);


--
-- Name: resource_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX resource_extra_perms ON public.resource USING gin (extra_perms);


--
-- Name: schedule_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX schedule_extra_perms ON public.schedule USING gin (extra_perms);


--
-- Name: script_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX script_extra_perms ON public.script USING gin (extra_perms);


--
-- Name: variable_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX variable_extra_perms ON public.variable USING gin (extra_perms);


--
-- Name: worker_ping_on_ping_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX worker_ping_on_ping_at ON public.worker_ping USING btree (ping_at);


--
-- Name: account account_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.account
    ADD CONSTRAINT account_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: app_version app_version_flow_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_version
    ADD CONSTRAINT app_version_flow_id_fkey FOREIGN KEY (app_id) REFERENCES public.app(id) ON DELETE CASCADE;


--
-- Name: app app_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app
    ADD CONSTRAINT app_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: capture capture_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.capture
    ADD CONSTRAINT capture_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: completed_job completed_job_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.completed_job
    ADD CONSTRAINT completed_job_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: draft draft_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.draft
    ADD CONSTRAINT draft_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: usr_to_group fk_group; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.usr_to_group
    ADD CONSTRAINT fk_group FOREIGN KEY (workspace_id, group_) REFERENCES public.group_(workspace_id, name);


--
-- Name: flow flow_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow
    ADD CONSTRAINT flow_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: folder folder_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.folder
    ADD CONSTRAINT folder_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id) ON DELETE CASCADE;


--
-- Name: group_ group__workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_
    ADD CONSTRAINT group__workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: input input_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.input
    ADD CONSTRAINT input_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: queue queue_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.queue
    ADD CONSTRAINT queue_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: raw_app raw_app_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.raw_app
    ADD CONSTRAINT raw_app_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: resource_type resource_type_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.resource_type
    ADD CONSTRAINT resource_type_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: resource resource_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.resource
    ADD CONSTRAINT resource_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: resume_job resume_job_flow_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.resume_job
    ADD CONSTRAINT resume_job_flow_fkey FOREIGN KEY (flow) REFERENCES public.queue(id) ON DELETE CASCADE;


--
-- Name: schedule schedule_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.schedule
    ADD CONSTRAINT schedule_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: script script_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.script
    ADD CONSTRAINT script_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: token token_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.token
    ADD CONSTRAINT token_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: usr_to_group usr_to_group_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.usr_to_group
    ADD CONSTRAINT usr_to_group_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: usr usr_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.usr
    ADD CONSTRAINT usr_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: variable variable_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.variable
    ADD CONSTRAINT variable_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: workspace_invite workspace_invite_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_invite
    ADD CONSTRAINT workspace_invite_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: workspace_key workspace_key_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_key
    ADD CONSTRAINT workspace_key_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: workspace_settings workspace_settings_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_settings
    ADD CONSTRAINT workspace_settings_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: account; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.account ENABLE ROW LEVEL SECURITY;

--
-- Name: app; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.app ENABLE ROW LEVEL SECURITY;

--
-- Name: audit; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.audit ENABLE ROW LEVEL SECURITY;

--
-- Name: capture; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.capture ENABLE ROW LEVEL SECURITY;

--
-- Name: completed_job; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.completed_job ENABLE ROW LEVEL SECURITY;

--
-- Name: flow; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.flow ENABLE ROW LEVEL SECURITY;

--
-- Name: folder; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.folder ENABLE ROW LEVEL SECURITY;

--
-- Name: queue; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.queue ENABLE ROW LEVEL SECURITY;

--
-- Name: raw_app; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.raw_app ENABLE ROW LEVEL SECURITY;

--
-- Name: resource; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.resource ENABLE ROW LEVEL SECURITY;

--
-- Name: audit schedule; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY schedule ON public.audit FOR INSERT WITH CHECK (((username)::text ~~ 'schedule-%'::text));


--
-- Name: schedule; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.schedule ENABLE ROW LEVEL SECURITY;

--
-- Name: script; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.script ENABLE ROW LEVEL SECURITY;

--
-- Name: app see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.app USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: flow see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.flow USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(flow.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: folder see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.folder USING (((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)) OR (EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))))))) WITH CHECK ((EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))))));


--
-- Name: raw_app see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.raw_app USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(raw_app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: resource see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.resource USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(resource.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: schedule see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.schedule USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(schedule.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: script see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.script USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(script.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: usr_to_group see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.usr_to_group USING (true) WITH CHECK ((EXISTS ( SELECT f.key,
    f.value
   FROM public.group_ g,
    LATERAL jsonb_each_text(g.extra_perms) f(key, value)
  WHERE (((usr_to_group.group_)::text = (g.name)::text) AND ((usr_to_group.workspace_id)::text = (g.workspace_id)::text) AND (split_part(f.key, '/'::text, 1) = 'g'::text) AND (f.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (f.value)::boolean))));


--
-- Name: variable see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.variable USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(variable.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: app see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.app USING ((extra_perms ? concat('u/', current_setting('session.user'::text)))) WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: flow see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.flow USING ((extra_perms ? concat('u/', current_setting('session.user'::text)))) WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: folder see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.folder USING (((extra_perms ? concat('u/', current_setting('session.user'::text))) OR (concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[])))) WITH CHECK ((concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[])));


--
-- Name: raw_app see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.raw_app USING ((extra_perms ? concat('u/', current_setting('session.user'::text)))) WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: resource see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.resource USING ((extra_perms ? concat('u/', current_setting('session.user'::text)))) WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: schedule see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.schedule USING ((extra_perms ? concat('u/', current_setting('session.user'::text)))) WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: script see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.script USING ((extra_perms ? concat('u/', current_setting('session.user'::text)))) WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: usr_to_group see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.usr_to_group USING (true) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.group_
  WHERE (((usr_to_group.group_)::text = (group_.name)::text) AND ((usr_to_group.workspace_id)::text = (group_.workspace_id)::text) AND ((group_.extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean))));


--
-- Name: variable see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.variable USING ((extra_perms ? concat('u/', current_setting('session.user'::text)))) WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: account see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.account USING (((split_part((owner)::text, '/'::text, 1) = 'f'::text) AND (split_part((owner)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text))))) WITH CHECK (((split_part((owner)::text, '/'::text, 1) = 'f'::text) AND (split_part((owner)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: app see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.app USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text))))) WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.capture USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text))))) WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: completed_job see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.completed_job USING (((visible_to_owner IS TRUE) AND (split_part((script_path)::text, '/'::text, 1) = 'f'::text) AND (split_part((script_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: flow see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.flow USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text))))) WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: queue see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.queue USING (((visible_to_owner IS TRUE) AND (split_part((script_path)::text, '/'::text, 1) = 'f'::text) AND (split_part((script_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: raw_app see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.raw_app USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text))))) WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: resource see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.resource USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text))))) WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: schedule see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.schedule USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text))))) WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: script see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.script USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text))))) WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: variable see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.variable USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text))))) WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: account see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.account USING (((split_part((owner)::text, '/'::text, 1) = 'g'::text) AND (split_part((owner)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: app see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.app USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: capture see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.capture USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: completed_job see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.completed_job USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'g'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: flow see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.flow USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: queue see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.queue USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'g'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: raw_app see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.raw_app USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: resource see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.resource USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: schedule see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.schedule USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: script see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.script USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: variable see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.variable USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: completed_job see_member_path; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member_path ON public.completed_job USING (((visible_to_owner IS TRUE) AND (split_part((script_path)::text, '/'::text, 1) = 'g'::text) AND (split_part((script_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: queue see_member_path; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member_path ON public.queue USING (((visible_to_owner IS TRUE) AND (split_part((script_path)::text, '/'::text, 1) = 'g'::text) AND (split_part((script_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: account see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.account USING (((split_part((owner)::text, '/'::text, 1) = 'u'::text) AND (split_part((owner)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: app see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.app USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: audit see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.audit USING (((username)::text = current_setting('session.user'::text)));


--
-- Name: capture see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.capture USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: completed_job see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.completed_job USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'u'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: flow see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.flow USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: queue see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.queue USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'u'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: raw_app see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.raw_app USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: resource see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.resource USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: schedule see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.schedule USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: script see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.script USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: variable see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.variable USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: completed_job see_own_path; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own_path ON public.completed_job USING (((visible_to_owner IS TRUE) AND (split_part((script_path)::text, '/'::text, 1) = 'u'::text) AND (split_part((script_path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: queue see_own_path; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own_path ON public.queue USING (((visible_to_owner IS TRUE) AND (split_part((script_path)::text, '/'::text, 1) = 'u'::text) AND (split_part((script_path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: usr_to_group; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.usr_to_group ENABLE ROW LEVEL SECURITY;

--
-- Name: variable; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.variable ENABLE ROW LEVEL SECURITY;

--
-- Name: TABLE _sqlx_migrations; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public._sqlx_migrations TO windmill_user;
GRANT ALL ON TABLE public._sqlx_migrations TO windmill_admin;


--
-- Name: TABLE account; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.account TO windmill_user;
GRANT ALL ON TABLE public.account TO windmill_admin;


--
-- Name: SEQUENCE account_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.account_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.account_id_seq TO windmill_admin;


--
-- Name: TABLE app; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.app TO windmill_user;
GRANT ALL ON TABLE public.app TO windmill_admin;


--
-- Name: SEQUENCE app_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.app_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.app_id_seq TO windmill_admin;


--
-- Name: TABLE app_version; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.app_version TO windmill_user;
GRANT ALL ON TABLE public.app_version TO windmill_admin;


--
-- Name: SEQUENCE app_version_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.app_version_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.app_version_id_seq TO windmill_admin;


--
-- Name: TABLE audit; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.audit TO windmill_user;
GRANT ALL ON TABLE public.audit TO windmill_admin;


--
-- Name: SEQUENCE audit_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.audit_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.audit_id_seq TO windmill_admin;


--
-- Name: TABLE capture; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.capture TO windmill_user;
GRANT ALL ON TABLE public.capture TO windmill_admin;


--
-- Name: TABLE completed_job; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.completed_job TO windmill_user;
GRANT ALL ON TABLE public.completed_job TO windmill_admin;


--
-- Name: TABLE demo; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.demo TO windmill_user;


--
-- Name: TABLE draft; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.draft TO windmill_user;
GRANT ALL ON TABLE public.draft TO windmill_admin;


--
-- Name: TABLE email_to_igroup; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.email_to_igroup TO windmill_user;
GRANT ALL ON TABLE public.email_to_igroup TO windmill_admin;


--
-- Name: TABLE favorite; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.favorite TO windmill_user;
GRANT ALL ON TABLE public.favorite TO windmill_admin;


--
-- Name: TABLE flow; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.flow TO windmill_user;
GRANT ALL ON TABLE public.flow TO windmill_admin;


--
-- Name: TABLE folder; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.folder TO windmill_user;
GRANT ALL ON TABLE public.folder TO windmill_admin;


--
-- Name: TABLE group_; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.group_ TO windmill_user;
GRANT ALL ON TABLE public.group_ TO windmill_admin;


--
-- Name: TABLE input; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.input TO windmill_user;
GRANT ALL ON TABLE public.input TO windmill_admin;


--
-- Name: TABLE instance_group; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.instance_group TO windmill_user;
GRANT ALL ON TABLE public.instance_group TO windmill_admin;


--
-- Name: TABLE magic_link; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.magic_link TO windmill_user;
GRANT ALL ON TABLE public.magic_link TO windmill_admin;


--
-- Name: TABLE password; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.password TO windmill_user;
GRANT ALL ON TABLE public.password TO windmill_admin;


--
-- Name: TABLE pip_resolution_cache; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.pip_resolution_cache TO windmill_user;


--
-- Name: TABLE queue; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.queue TO windmill_user;
GRANT ALL ON TABLE public.queue TO windmill_admin;


--
-- Name: TABLE raw_app; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.raw_app TO windmill_user;
GRANT ALL ON TABLE public.raw_app TO windmill_admin;


--
-- Name: TABLE resource; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.resource TO windmill_user;
GRANT ALL ON TABLE public.resource TO windmill_admin;


--
-- Name: TABLE resource_type; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.resource_type TO windmill_user;
GRANT ALL ON TABLE public.resource_type TO windmill_admin;


--
-- Name: TABLE resume_job; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.resume_job TO windmill_user;
GRANT ALL ON TABLE public.resume_job TO windmill_admin;


--
-- Name: TABLE schedule; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.schedule TO windmill_user;
GRANT ALL ON TABLE public.schedule TO windmill_admin;


--
-- Name: TABLE script; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.script TO windmill_user;
GRANT ALL ON TABLE public.script TO windmill_admin;


--
-- Name: TABLE token; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.token TO windmill_user;
GRANT ALL ON TABLE public.token TO windmill_admin;


--
-- Name: TABLE usage; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.usage TO windmill_user;
GRANT ALL ON TABLE public.usage TO windmill_admin;


--
-- Name: TABLE usr; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.usr TO windmill_user;
GRANT ALL ON TABLE public.usr TO windmill_admin;


--
-- Name: TABLE usr_to_group; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.usr_to_group TO windmill_user;
GRANT ALL ON TABLE public.usr_to_group TO windmill_admin;


--
-- Name: TABLE variable; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.variable TO windmill_user;
GRANT ALL ON TABLE public.variable TO windmill_admin;


--
-- Name: TABLE worker_ping; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.worker_ping TO windmill_user;
GRANT ALL ON TABLE public.worker_ping TO windmill_admin;


--
-- Name: TABLE workspace; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.workspace TO windmill_user;
GRANT ALL ON TABLE public.workspace TO windmill_admin;


--
-- Name: TABLE workspace_invite; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.workspace_invite TO windmill_user;
GRANT ALL ON TABLE public.workspace_invite TO windmill_admin;


--
-- Name: TABLE workspace_key; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.workspace_key TO windmill_user;
GRANT ALL ON TABLE public.workspace_key TO windmill_admin;


--
-- Name: TABLE workspace_settings; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.workspace_settings TO windmill_user;
GRANT ALL ON TABLE public.workspace_settings TO windmill_admin;


--
-- Name: DEFAULT PRIVILEGES FOR SEQUENCES; Type: DEFAULT ACL; Schema: public; Owner: postgres
--

ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL ON SEQUENCES  TO windmill_user;


--
-- Name: DEFAULT PRIVILEGES FOR TABLES; Type: DEFAULT ACL; Schema: public; Owner: postgres
--

ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL ON TABLES  TO windmill_user;


--
-- PostgreSQL database dump complete
--



-- used for backend automated testing
-- https://docs.rs/sqlx/latest/sqlx/attr.test.html


INSERT INTO public.workspace
            (id,               name,             owner)
     VALUES ('test-workspace', 'test-workspace', 'test-user');

INSERT INTO public.usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test@windmill.dev', 'test-user', true, 'Admin');

INSERT INTO public.workspace_key(workspace_id, kind, key) VALUES
	('test-workspace', 'cloud', 'test-key');

insert INTO public.token(token, email, label, super_admin) VALUES ('SECRET_TOKEN', 'test@windmill.dev', 'test token', true);

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'system',
'
import pandas as pd
import pandas2
',
'{}',
'',
'',
'f/foo/bar', -28028598712388162, 'python3', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'system',
'
import numpy as np
',
'{}',
'',
'',
'f/foo/baz', -28028598712388161, 'python3', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'system',
'
import difffolder
from .bar import main
',
'{}',
'',
'',
'f/foobar/baz', -28028598712388160, 'python3', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'system',
'
import innerdifffolder
',
'{}',
'',
'',
'f/foobar/bar', -28028598712388159, 'python3', '');
