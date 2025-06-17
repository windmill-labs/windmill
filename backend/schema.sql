--
-- PostgreSQL database dump
--

-- Dumped from database version 16.8 (Debian 16.8-1.pgdg120+1)
-- Dumped by pg_dump version 16.8 (Debian 16.8-1.pgdg120+1)

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
-- Name: public; Type: SCHEMA; Schema: -; Owner: pg_database_owner
--

CREATE SCHEMA public;


ALTER SCHEMA public OWNER TO pg_database_owner;

--
-- Name: SCHEMA public; Type: COMMENT; Schema: -; Owner: pg_database_owner
--

COMMENT ON SCHEMA public IS 'standard public schema';


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
-- Name: authentication_method; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.authentication_method AS ENUM (
    'none',
    'windmill',
    'api_key',
    'basic_http',
    'custom_script',
    'signature'
);


ALTER TYPE public.authentication_method OWNER TO postgres;

--
-- Name: autoscaling_event_type; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.autoscaling_event_type AS ENUM (
    'full_scaleout',
    'scalein',
    'scaleout'
);


ALTER TYPE public.autoscaling_event_type OWNER TO postgres;

--
-- Name: aws_auth_resource_type; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.aws_auth_resource_type AS ENUM (
    'oidc',
    'credentials'
);


ALTER TYPE public.aws_auth_resource_type OWNER TO postgres;

--
-- Name: delivery_mode; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.delivery_mode AS ENUM (
    'push',
    'pull'
);


ALTER TYPE public.delivery_mode OWNER TO postgres;

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
-- Name: gcp_subscription_mode; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.gcp_subscription_mode AS ENUM (
    'create_update',
    'existing'
);


ALTER TYPE public.gcp_subscription_mode OWNER TO postgres;

--
-- Name: http_method; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.http_method AS ENUM (
    'get',
    'post',
    'put',
    'delete',
    'patch'
);


ALTER TYPE public.http_method OWNER TO postgres;

--
-- Name: importer_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.importer_kind AS ENUM (
    'script',
    'flow',
    'app'
);


ALTER TYPE public.importer_kind OWNER TO postgres;

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
    'noop',
    'appdependencies',
    'deploymentcallback',
    'singlescriptflow',
    'flowscript',
    'flownode',
    'appscript'
);


ALTER TYPE public.job_kind OWNER TO postgres;

--
-- Name: job_status; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.job_status AS ENUM (
    'success',
    'failure',
    'canceled',
    'skipped'
);


ALTER TYPE public.job_status OWNER TO postgres;

--
-- Name: job_trigger_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.job_trigger_kind AS ENUM (
    'webhook',
    'http',
    'websocket',
    'kafka',
    'email',
    'nats',
    'schedule',
    'app',
    'ui',
    'postgres',
    'sqs',
    'gcp'
);


ALTER TYPE public.job_trigger_kind OWNER TO postgres;

--
-- Name: log_mode; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.log_mode AS ENUM (
    'standalone',
    'server',
    'worker',
    'agent',
    'indexer',
    'mcp'
);


ALTER TYPE public.log_mode OWNER TO postgres;

--
-- Name: login_type; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.login_type AS ENUM (
    'password',
    'github'
);


ALTER TYPE public.login_type OWNER TO postgres;

--
-- Name: metric_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.metric_kind AS ENUM (
    'scalar_int',
    'scalar_float',
    'timeseries_int',
    'timeseries_float'
);


ALTER TYPE public.metric_kind OWNER TO postgres;

--
-- Name: mqtt_client_version; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.mqtt_client_version AS ENUM (
    'v3',
    'v5'
);


ALTER TYPE public.mqtt_client_version OWNER TO postgres;

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
    'approval',
    'preprocessor'
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
    'bun',
    'mysql',
    'bigquery',
    'snowflake',
    'graphql',
    'powershell',
    'mssql',
    'php',
    'bunnative',
    'rust',
    'ansible',
    'csharp',
    'oracledb',
    'nu',
    'java',
    'duckdb'
);


ALTER TYPE public.script_lang OWNER TO postgres;

--
-- Name: trigger_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.trigger_kind AS ENUM (
    'webhook',
    'http',
    'websocket',
    'kafka',
    'email',
    'nats',
    'postgres',
    'sqs',
    'mqtt',
    'gcp'
);


ALTER TYPE public.trigger_kind OWNER TO postgres;

--
-- Name: workspace_key_kind; Type: TYPE; Schema: public; Owner: postgres
--

CREATE TYPE public.workspace_key_kind AS ENUM (
    'cloud'
);


ALTER TYPE public.workspace_key_kind OWNER TO postgres;

--
-- Name: notify_config_change(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.notify_config_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_config_change', NEW.name::text);
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.notify_config_change() OWNER TO postgres;

--
-- Name: notify_global_setting_change(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.notify_global_setting_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_global_setting_change', NEW.name::text);
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.notify_global_setting_change() OWNER TO postgres;

--
-- Name: notify_global_setting_delete(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.notify_global_setting_delete() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_global_setting_change', OLD.name::text);
    RETURN OLD;
END;
$$;


ALTER FUNCTION public.notify_global_setting_delete() OWNER TO postgres;

--
-- Name: notify_http_trigger_change(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.notify_http_trigger_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_http_trigger_change', NEW.workspace_id || ':' || NEW.path);
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.notify_http_trigger_change() OWNER TO postgres;

--
-- Name: notify_runnable_version_change(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.notify_runnable_version_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    source_type TEXT;
    kind TEXT;
BEGIN
    source_type := TG_ARGV[0];

    IF source_type = 'script' THEN
        kind := NEW.kind;
    ELSE
        kind := 'flow';
    END IF;

    PERFORM pg_notify('notify_runnable_version_change', NEW.workspace_id || ':' || source_type || ':' || NEW.path || ':' || kind);
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.notify_runnable_version_change() OWNER TO postgres;

--
-- Name: notify_token_invalidation(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.notify_token_invalidation() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    -- Only notify for session token deletions when the invalidation settings are enabled
    IF OLD.label = 'session' AND OLD.email IS NOT NULL THEN
        PERFORM pg_notify('notify_token_invalidation', OLD.token);
    END IF;
    RETURN OLD;
END;
$$;


ALTER FUNCTION public.notify_token_invalidation() OWNER TO postgres;

--
-- Name: notify_webhook_change(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.notify_webhook_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_webhook_change', NEW.workspace_id);
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.notify_webhook_change() OWNER TO postgres;

--
-- Name: notify_workspace_envs_change(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.notify_workspace_envs_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_envs_change', NEW.workspace_id);
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.notify_workspace_envs_change() OWNER TO postgres;

--
-- Name: notify_workspace_premium_change(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.notify_workspace_premium_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_premium_change', NEW.id);
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.notify_workspace_premium_change() OWNER TO postgres;

--
-- Name: prevent_route_path_change(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.prevent_route_path_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF CURRENT_USER = 'windmill_user' AND NEW.route_path <> OLD.route_path THEN
        RAISE EXCEPTION 'Modification of route_path is only allowed by admins';
    END IF;
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.prevent_route_path_change() OWNER TO postgres;

--
-- Name: set_session_context(boolean, text, text, text, text, text); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.set_session_context(admin boolean, username text, groups text, pgroups text, folders_read text, folders_write text) RETURNS void
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF admin THEN
        SET LOCAL ROLE windmill_admin;
    ELSE
        SET LOCAL ROLE windmill_user;
    END IF;
    PERFORM set_config('session.user', username, true);
    PERFORM set_config('session.groups', groups, true);
    PERFORM set_config('session.pgroups', pgroups, true);
    PERFORM set_config('session.folders_read', folders_read, true);
    PERFORM set_config('session.folders_write', folders_write, true);
END;
$$;


ALTER FUNCTION public.set_session_context(admin boolean, username text, groups text, pgroups text, folders_read text, folders_write text) OWNER TO postgres;

SET default_tablespace = '';

SET default_table_access_method = heap;

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


ALTER SEQUENCE public.account_id_seq OWNER TO postgres;

--
-- Name: account_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.account_id_seq OWNED BY public.account.id;


--
-- Name: alerts; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.alerts (
    id integer NOT NULL,
    alert_type character varying(50) NOT NULL,
    message text NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    acknowledged boolean,
    workspace_id text,
    acknowledged_workspace boolean,
    resource text
);


ALTER TABLE public.alerts OWNER TO postgres;

--
-- Name: alerts_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.alerts_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.alerts_id_seq OWNER TO postgres;

--
-- Name: alerts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.alerts_id_seq OWNED BY public.alerts.id;


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
    draft_only boolean,
    custom_path text,
    CONSTRAINT app_custom_path_check CHECK ((custom_path ~ '^[\w-]+(\/[\w-]+)*$'::text))
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


ALTER SEQUENCE public.app_id_seq OWNER TO postgres;

--
-- Name: app_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.app_id_seq OWNED BY public.app.id;


--
-- Name: app_script; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.app_script (
    id bigint NOT NULL,
    app bigint NOT NULL,
    hash character(64) NOT NULL,
    lock text,
    code text NOT NULL,
    code_sha256 character(64) NOT NULL
);


ALTER TABLE public.app_script OWNER TO postgres;

--
-- Name: app_script_app_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.app_script_app_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.app_script_app_seq OWNER TO postgres;

--
-- Name: app_script_app_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.app_script_app_seq OWNED BY public.app_script.app;


--
-- Name: app_script_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.app_script_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.app_script_id_seq OWNER TO postgres;

--
-- Name: app_script_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.app_script_id_seq OWNED BY public.app_script.id;


--
-- Name: app_version; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.app_version (
    id bigint NOT NULL,
    app_id bigint NOT NULL,
    value json NOT NULL,
    created_by character varying(50) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    raw_app boolean DEFAULT false NOT NULL
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


ALTER SEQUENCE public.app_version_id_seq OWNER TO postgres;

--
-- Name: app_version_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.app_version_id_seq OWNED BY public.app_version.id;


--
-- Name: app_version_lite; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.app_version_lite (
    id bigint NOT NULL,
    value jsonb
);


ALTER TABLE public.app_version_lite OWNER TO postgres;

--
-- Name: app_version_lite_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.app_version_lite_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.app_version_lite_id_seq OWNER TO postgres;

--
-- Name: app_version_lite_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.app_version_lite_id_seq OWNED BY public.app_version_lite.id;


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


ALTER SEQUENCE public.audit_id_seq OWNER TO postgres;

--
-- Name: audit_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.audit_id_seq OWNED BY public.audit.id;


--
-- Name: autoscaling_event; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.autoscaling_event (
    id integer NOT NULL,
    worker_group text NOT NULL,
    event_type public.autoscaling_event_type NOT NULL,
    desired_workers integer NOT NULL,
    applied_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    reason text
);


ALTER TABLE public.autoscaling_event OWNER TO postgres;

--
-- Name: autoscaling_event_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.autoscaling_event_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.autoscaling_event_id_seq OWNER TO postgres;

--
-- Name: autoscaling_event_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.autoscaling_event_id_seq OWNED BY public.autoscaling_event.id;


--
-- Name: capture; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.capture (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by character varying(50) NOT NULL,
    main_args jsonb DEFAULT 'null'::jsonb NOT NULL,
    is_flow boolean NOT NULL,
    trigger_kind public.trigger_kind NOT NULL,
    preprocessor_args jsonb,
    id bigint NOT NULL,
    CONSTRAINT capture_payload_too_big CHECK ((length((main_args)::text) < (512 * 1024)))
);


ALTER TABLE public.capture OWNER TO postgres;

--
-- Name: capture_config; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.capture_config (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    trigger_kind public.trigger_kind NOT NULL,
    trigger_config jsonb,
    owner character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    server_id character varying(50),
    last_client_ping timestamp with time zone,
    last_server_ping timestamp with time zone,
    error text
);


ALTER TABLE public.capture_config OWNER TO postgres;

--
-- Name: capture_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

ALTER TABLE public.capture ALTER COLUMN id ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME public.capture_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: cloud_workspace_settings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.cloud_workspace_settings (
    workspace_id character varying(50) NOT NULL,
    threshold_alert_amount integer,
    last_alert_sent timestamp without time zone,
    last_warning_sent timestamp without time zone
);


ALTER TABLE public.cloud_workspace_settings OWNER TO postgres;

--
-- Name: concurrency_counter; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.concurrency_counter (
    concurrency_id character varying(1000) NOT NULL,
    job_uuids jsonb DEFAULT '{}'::jsonb NOT NULL
);


ALTER TABLE public.concurrency_counter OWNER TO postgres;

--
-- Name: concurrency_key; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.concurrency_key (
    key character varying(255) NOT NULL,
    ended_at timestamp with time zone,
    job_id uuid NOT NULL
);


ALTER TABLE public.concurrency_key OWNER TO postgres;

--
-- Name: concurrency_locks; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.concurrency_locks (
    id character varying NOT NULL,
    last_locked_at timestamp without time zone NOT NULL,
    owner character varying
);


ALTER TABLE public.concurrency_locks OWNER TO postgres;

--
-- Name: config; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.config (
    name character varying(255) NOT NULL,
    config jsonb DEFAULT '{}'::jsonb
);


ALTER TABLE public.config OWNER TO postgres;

--
-- Name: custom_concurrency_key_ended; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.custom_concurrency_key_ended (
    key character varying(255) NOT NULL,
    ended_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.custom_concurrency_key_ended OWNER TO postgres;

--
-- Name: dependency_map; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.dependency_map (
    workspace_id character varying(50) NOT NULL,
    importer_path character varying(510) NOT NULL,
    importer_kind public.importer_kind NOT NULL,
    imported_path character varying(510) NOT NULL,
    importer_node_id character varying(255) DEFAULT ''::character varying NOT NULL
);


ALTER TABLE public.dependency_map OWNER TO postgres;

--
-- Name: deployment_metadata; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.deployment_metadata (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    script_hash bigint,
    app_version bigint,
    callback_job_ids uuid[],
    deployment_msg text,
    flow_version bigint
);


ALTER TABLE public.deployment_metadata OWNER TO postgres;

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
    ws_error_handler_muted boolean DEFAULT false NOT NULL,
    dedicated_worker boolean,
    timeout integer,
    visible_to_runner_only boolean,
    concurrency_key character varying(255),
    versions bigint[] DEFAULT '{}'::bigint[] NOT NULL,
    on_behalf_of_email text,
    lock_error_logs text,
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


ALTER TABLE public.flow OWNER TO postgres;

--
-- Name: flow_node_hash_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.flow_node_hash_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.flow_node_hash_seq OWNER TO postgres;

--
-- Name: flow_node; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.flow_node (
    id bigint NOT NULL,
    workspace_id character varying(50) NOT NULL,
    hash bigint,
    path character varying(255) NOT NULL,
    lock text,
    code text,
    flow jsonb,
    hash_v2 character(64) DEFAULT to_hex(nextval('public.flow_node_hash_seq'::regclass)) NOT NULL
);


ALTER TABLE public.flow_node OWNER TO postgres;

--
-- Name: flow_node_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.flow_node_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.flow_node_id_seq OWNER TO postgres;

--
-- Name: flow_node_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.flow_node_id_seq OWNED BY public.flow_node.id;


--
-- Name: flow_version; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.flow_version (
    id bigint NOT NULL,
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    value jsonb NOT NULL,
    schema json,
    created_by character varying(50) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.flow_version OWNER TO postgres;

--
-- Name: flow_version_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.flow_version_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.flow_version_id_seq OWNER TO postgres;

--
-- Name: flow_version_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.flow_version_id_seq OWNED BY public.flow_version.id;


--
-- Name: flow_version_lite; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.flow_version_lite (
    id bigint NOT NULL,
    value jsonb
);


ALTER TABLE public.flow_version_lite OWNER TO postgres;

--
-- Name: flow_version_lite_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.flow_version_lite_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.flow_version_lite_id_seq OWNER TO postgres;

--
-- Name: flow_version_lite_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.flow_version_lite_id_seq OWNED BY public.flow_version_lite.id;


--
-- Name: workspace_runnable_dependencies; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.workspace_runnable_dependencies (
    flow_path character varying(255),
    runnable_path character varying(255) NOT NULL,
    script_hash bigint,
    runnable_is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    app_path character varying(255),
    CONSTRAINT workspace_runnable_dependencies_path_exclusive CHECK ((((flow_path IS NOT NULL) AND (app_path IS NULL)) OR ((flow_path IS NULL) AND (app_path IS NOT NULL))))
);


ALTER TABLE public.workspace_runnable_dependencies OWNER TO postgres;

--
-- Name: flow_workspace_runnables; Type: VIEW; Schema: public; Owner: postgres
--

CREATE VIEW public.flow_workspace_runnables AS
 SELECT flow_path,
    runnable_path,
    script_hash,
    runnable_is_flow,
    workspace_id
   FROM public.workspace_runnable_dependencies;


ALTER VIEW public.flow_workspace_runnables OWNER TO postgres;

--
-- Name: folder; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.folder (
    name character varying(255) NOT NULL,
    workspace_id character varying(50) NOT NULL,
    display_name character varying(100) NOT NULL,
    owners character varying(255)[] NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    summary text,
    edited_at timestamp with time zone,
    created_by character varying(50)
);


ALTER TABLE public.folder OWNER TO postgres;

--
-- Name: gcp_trigger; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.gcp_trigger (
    gcp_resource_path character varying(255) NOT NULL,
    topic_id character varying(255) NOT NULL,
    subscription_id character varying(255) NOT NULL,
    delivery_type public.delivery_mode NOT NULL,
    delivery_config jsonb,
    path character varying(255) NOT NULL,
    script_path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    edited_by character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    server_id character varying(50),
    last_server_ping timestamp with time zone,
    error text,
    enabled boolean NOT NULL,
    subscription_mode public.gcp_subscription_mode DEFAULT 'create_update'::public.gcp_subscription_mode NOT NULL,
    CONSTRAINT gcp_trigger_check CHECK (((delivery_type <> 'push'::public.delivery_mode) OR (delivery_config IS NOT NULL))),
    CONSTRAINT gcp_trigger_subscription_id_check CHECK (((char_length((subscription_id)::text) >= 3) AND (char_length((subscription_id)::text) <= 255))),
    CONSTRAINT gcp_trigger_topic_id_check CHECK (((char_length((topic_id)::text) >= 3) AND (char_length((topic_id)::text) <= 255)))
);


ALTER TABLE public.gcp_trigger OWNER TO postgres;

--
-- Name: global_settings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.global_settings (
    name character varying(255) NOT NULL,
    value jsonb NOT NULL,
    updated_at timestamp without time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.global_settings OWNER TO postgres;

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
-- Name: healthchecks; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.healthchecks (
    id bigint NOT NULL,
    check_type character varying(50) NOT NULL,
    healthy boolean NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


ALTER TABLE public.healthchecks OWNER TO postgres;

--
-- Name: healthchecks_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.healthchecks_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.healthchecks_id_seq OWNER TO postgres;

--
-- Name: healthchecks_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.healthchecks_id_seq OWNED BY public.healthchecks.id;


--
-- Name: http_trigger; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.http_trigger (
    path character varying(255) NOT NULL,
    route_path character varying(255) NOT NULL,
    route_path_key character varying(255) NOT NULL,
    script_path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    edited_by character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    is_async boolean DEFAULT false NOT NULL,
    authentication_method public.authentication_method DEFAULT 'none'::public.authentication_method NOT NULL,
    http_method public.http_method NOT NULL,
    static_asset_config jsonb,
    is_static_website boolean DEFAULT false NOT NULL,
    workspaced_route boolean DEFAULT false NOT NULL,
    wrap_body boolean DEFAULT false NOT NULL,
    raw_string boolean DEFAULT false NOT NULL,
    authentication_resource_path character varying(255) DEFAULT NULL::character varying
);


ALTER TABLE public.http_trigger OWNER TO postgres;

--
-- Name: http_trigger_version_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.http_trigger_version_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.http_trigger_version_seq OWNER TO postgres;

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
    id character varying(1000),
    scim_display_name character varying(255),
    external_id character varying(512)
);


ALTER TABLE public.instance_group OWNER TO postgres;

--
-- Name: job_logs; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.job_logs (
    job_id uuid NOT NULL,
    workspace_id character varying(255),
    created_at timestamp with time zone DEFAULT now(),
    logs text,
    log_offset integer DEFAULT 0 NOT NULL,
    log_file_index text[]
);


ALTER TABLE public.job_logs OWNER TO postgres;

--
-- Name: job_perms; Type: TABLE; Schema: public; Owner: postgres
--

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

--
-- Name: job_stats; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.job_stats (
    workspace_id character varying(50) NOT NULL,
    job_id uuid NOT NULL,
    metric_id character varying(50) NOT NULL,
    metric_name character varying(255),
    metric_kind public.metric_kind NOT NULL,
    scalar_int integer,
    scalar_float real,
    timestamps timestamp with time zone[],
    timeseries_int integer[],
    timeseries_float real[]
);


ALTER TABLE public.job_stats OWNER TO postgres;

--
-- Name: kafka_trigger; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.kafka_trigger (
    path character varying(255) NOT NULL,
    kafka_resource_path character varying(255) NOT NULL,
    topics character varying(255)[] NOT NULL,
    group_id character varying(255) NOT NULL,
    script_path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    edited_by character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    server_id character varying(50),
    last_server_ping timestamp with time zone,
    error text,
    enabled boolean NOT NULL
);


ALTER TABLE public.kafka_trigger OWNER TO postgres;

--
-- Name: log_file; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.log_file (
    hostname character varying(255) NOT NULL,
    log_ts timestamp without time zone NOT NULL,
    ok_lines bigint,
    err_lines bigint,
    mode public.log_mode NOT NULL,
    worker_group character varying(255),
    file_path character varying(510) NOT NULL,
    json_fmt boolean DEFAULT false
);


ALTER TABLE public.log_file OWNER TO postgres;

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
-- Name: metrics; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.metrics (
    id character varying(255) NOT NULL,
    value jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


ALTER TABLE public.metrics OWNER TO postgres;

--
-- Name: mqtt_trigger; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.mqtt_trigger (
    mqtt_resource_path character varying(255) NOT NULL,
    subscribe_topics jsonb[] NOT NULL,
    client_version public.mqtt_client_version DEFAULT 'v5'::public.mqtt_client_version NOT NULL,
    v5_config jsonb,
    v3_config jsonb,
    client_id character varying(65535) DEFAULT NULL::character varying,
    path character varying(255) NOT NULL,
    script_path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    edited_by character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    server_id character varying(50),
    last_server_ping timestamp with time zone,
    error text,
    enabled boolean NOT NULL
);


ALTER TABLE public.mqtt_trigger OWNER TO postgres;

--
-- Name: nats_trigger; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.nats_trigger (
    path character varying(255) NOT NULL,
    nats_resource_path character varying(255) NOT NULL,
    subjects character varying(255)[] NOT NULL,
    stream_name character varying(255),
    consumer_name character varying(255),
    use_jetstream boolean NOT NULL,
    script_path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    edited_by character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    server_id character varying(50),
    last_server_ping timestamp with time zone,
    error text,
    enabled boolean NOT NULL
);


ALTER TABLE public.nats_trigger OWNER TO postgres;

--
-- Name: outstanding_wait_time; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.outstanding_wait_time (
    job_id uuid NOT NULL,
    self_wait_time_ms bigint,
    aggregate_wait_time_ms bigint
);


ALTER TABLE public.outstanding_wait_time OWNER TO postgres;

--
-- Name: parallel_monitor_lock; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.parallel_monitor_lock (
    parent_flow_id uuid NOT NULL,
    job_id uuid NOT NULL,
    last_ping timestamp with time zone
);


ALTER TABLE public.parallel_monitor_lock OWNER TO postgres;

--
-- Name: password; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.password (
    email character varying(255) NOT NULL,
    password_hash character varying(100),
    login_type character varying(50) NOT NULL,
    super_admin boolean DEFAULT false NOT NULL,
    verified boolean DEFAULT false NOT NULL,
    name character varying(255),
    company character varying(255),
    first_time_user boolean DEFAULT false NOT NULL,
    username character varying(50),
    devops boolean DEFAULT false NOT NULL
);


ALTER TABLE public.password OWNER TO postgres;

--
-- Name: pending_user; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.pending_user (
    email character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    username character varying(50) NOT NULL
);


ALTER TABLE public.pending_user OWNER TO postgres;

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
-- Name: postgres_trigger; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.postgres_trigger (
    path character varying(255) NOT NULL,
    script_path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    edited_by character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    extra_perms jsonb,
    postgres_resource_path character varying(255) NOT NULL,
    error text,
    server_id character varying(50),
    last_server_ping timestamp with time zone,
    replication_slot_name character varying(255) NOT NULL,
    publication_name character varying(255) NOT NULL,
    enabled boolean NOT NULL
);


ALTER TABLE public.postgres_trigger OWNER TO postgres;

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
    edited_at timestamp with time zone,
    created_by character varying(50),
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
    edited_at timestamp with time zone,
    created_by character varying(50),
    format_extension character varying(20),
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
    approver character varying(1000),
    resume_id integer DEFAULT 0 NOT NULL,
    approved boolean DEFAULT true NOT NULL
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
    on_recovery character varying(1000),
    on_failure_times integer,
    on_failure_exact boolean,
    on_failure_extra_args jsonb,
    on_recovery_times integer,
    on_recovery_extra_args jsonb,
    ws_error_handler_muted boolean DEFAULT false NOT NULL,
    retry jsonb,
    summary character varying(512),
    no_flow_overlap boolean DEFAULT false NOT NULL,
    tag character varying(50),
    paused_until timestamp with time zone,
    on_success character varying(1000),
    on_success_extra_args jsonb,
    cron_version text DEFAULT 'v1'::text,
    description text,
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
    cache_ttl integer,
    dedicated_worker boolean,
    ws_error_handler_muted boolean DEFAULT false NOT NULL,
    priority smallint,
    timeout integer,
    delete_after_use boolean,
    restart_unless_cancelled boolean,
    concurrency_key character varying(255),
    visible_to_runner_only boolean,
    no_main_func boolean,
    codebase character varying(255),
    has_preprocessor boolean,
    on_behalf_of_email text,
    schema_validation boolean DEFAULT false NOT NULL,
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


ALTER TABLE public.script OWNER TO postgres;

--
-- Name: sqs_trigger; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.sqs_trigger (
    path character varying(255) NOT NULL,
    queue_url character varying(255) NOT NULL,
    aws_resource_path character varying(255) NOT NULL,
    message_attributes text[],
    script_path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    edited_by character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    extra_perms jsonb,
    error text,
    server_id character varying(50),
    last_server_ping timestamp with time zone,
    enabled boolean NOT NULL,
    aws_auth_resource_type public.aws_auth_resource_type DEFAULT 'credentials'::public.aws_auth_resource_type NOT NULL
);


ALTER TABLE public.sqs_trigger OWNER TO postgres;

--
-- Name: token; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.token (
    token character varying(50) NOT NULL,
    label character varying(1000),
    expiration timestamp with time zone,
    workspace_id character varying(50),
    owner character varying(55),
    email character varying(255),
    super_admin boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone DEFAULT now() NOT NULL,
    scopes text[],
    job uuid
);


ALTER TABLE public.token OWNER TO postgres;

--
-- Name: tutorial_progress; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tutorial_progress (
    email character varying(255) NOT NULL,
    progress bit(64) DEFAULT '0'::"bit" NOT NULL
);


ALTER TABLE public.tutorial_progress OWNER TO postgres;

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
    email character varying(255) NOT NULL,
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
-- Name: v2_job; Type: TABLE; Schema: public; Owner: postgres
--

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


ALTER TABLE public.v2_job OWNER TO postgres;

--
-- Name: v2_job_completed; Type: TABLE; Schema: public; Owner: postgres
--

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


ALTER TABLE public.v2_job_completed OWNER TO postgres;

--
-- Name: v2_as_completed_job; Type: VIEW; Schema: public; Owner: postgres
--

CREATE VIEW public.v2_as_completed_job AS
 SELECT j.id,
    j.workspace_id,
    j.parent_job,
    j.created_by,
    j.created_at,
    c.duration_ms,
    ((c.status = 'success'::public.job_status) OR (c.status = 'skipped'::public.job_status)) AS success,
    j.runnable_id AS script_hash,
    j.runnable_path AS script_path,
    j.args,
    c.result,
    false AS deleted,
    j.raw_code,
    (c.status = 'canceled'::public.job_status) AS canceled,
    c.canceled_by,
    c.canceled_reason,
    j.kind AS job_kind,
        CASE
            WHEN (j.trigger_kind = 'schedule'::public.job_trigger_kind) THEN j.trigger
            ELSE NULL::character varying
        END AS schedule_path,
    j.permissioned_as,
    COALESCE(c.flow_status, c.workflow_as_code_status) AS flow_status,
    j.raw_flow,
    (j.flow_step_id IS NOT NULL) AS is_flow_step,
    j.script_lang AS language,
    c.started_at,
    (c.status = 'skipped'::public.job_status) AS is_skipped,
    j.raw_lock,
    j.permissioned_as_email AS email,
    j.visible_to_owner,
    c.memory_peak AS mem_peak,
    j.tag,
    j.priority,
    NULL::text AS logs,
    c.result_columns,
    j.script_entrypoint_override,
    j.preprocessed
   FROM (public.v2_job_completed c
     JOIN public.v2_job j USING (id));


ALTER VIEW public.v2_as_completed_job OWNER TO postgres;

--
-- Name: v2_job_queue; Type: TABLE; Schema: public; Owner: postgres
--

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


ALTER TABLE public.v2_job_queue OWNER TO postgres;

--
-- Name: v2_job_runtime; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.v2_job_runtime (
    id uuid NOT NULL,
    ping timestamp with time zone DEFAULT now(),
    memory_peak integer
);


ALTER TABLE public.v2_job_runtime OWNER TO postgres;

--
-- Name: v2_job_status; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.v2_job_status (
    id uuid NOT NULL,
    flow_status jsonb,
    flow_leaf_jobs jsonb,
    workflow_as_code_status jsonb
);


ALTER TABLE public.v2_job_status OWNER TO postgres;

--
-- Name: v2_as_queue; Type: VIEW; Schema: public; Owner: postgres
--

CREATE VIEW public.v2_as_queue AS
 SELECT j.id,
    j.workspace_id,
    j.parent_job,
    j.created_by,
    j.created_at,
    q.started_at,
    q.scheduled_for,
    q.running,
    j.runnable_id AS script_hash,
    j.runnable_path AS script_path,
    j.args,
    j.raw_code,
    (q.canceled_by IS NOT NULL) AS canceled,
    q.canceled_by,
    q.canceled_reason,
    r.ping AS last_ping,
    j.kind AS job_kind,
        CASE
            WHEN (j.trigger_kind = 'schedule'::public.job_trigger_kind) THEN j.trigger
            ELSE NULL::character varying
        END AS schedule_path,
    j.permissioned_as,
    COALESCE(s.flow_status, s.workflow_as_code_status) AS flow_status,
    j.raw_flow,
    (j.flow_step_id IS NOT NULL) AS is_flow_step,
    j.script_lang AS language,
    q.suspend,
    q.suspend_until,
    j.same_worker,
    j.raw_lock,
    j.pre_run_error,
    j.permissioned_as_email AS email,
    j.visible_to_owner,
    r.memory_peak AS mem_peak,
    j.flow_innermost_root_job AS root_job,
    s.flow_leaf_jobs AS leaf_jobs,
    j.tag,
    j.concurrent_limit,
    j.concurrency_time_window_s,
    j.timeout,
    j.flow_step_id,
    j.cache_ttl,
    j.priority,
    NULL::text AS logs,
    j.script_entrypoint_override,
    j.preprocessed
   FROM (((public.v2_job_queue q
     JOIN public.v2_job j USING (id))
     LEFT JOIN public.v2_job_runtime r USING (id))
     LEFT JOIN public.v2_job_status s USING (id));


ALTER VIEW public.v2_as_queue OWNER TO postgres;

--
-- Name: variable; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.variable (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    value character varying(15000) NOT NULL,
    is_secret boolean DEFAULT false NOT NULL,
    description character varying(10000) DEFAULT ''::character varying NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    account integer,
    is_oauth boolean DEFAULT false NOT NULL,
    expires_at timestamp with time zone,
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


ALTER TABLE public.variable OWNER TO postgres;

--
-- Name: websocket_trigger; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.websocket_trigger (
    path character varying(255) NOT NULL,
    url character varying(1000) NOT NULL,
    script_path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    edited_by character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    server_id character varying(50),
    last_server_ping timestamp with time zone,
    error text,
    enabled boolean NOT NULL,
    filters jsonb[] DEFAULT '{}'::jsonb[] NOT NULL,
    initial_messages jsonb[] DEFAULT '{}'::jsonb[],
    url_runnable_args jsonb DEFAULT '{}'::jsonb,
    can_return_message boolean DEFAULT false NOT NULL
);


ALTER TABLE public.websocket_trigger OWNER TO postgres;

--
-- Name: windmill_migrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.windmill_migrations (
    name text NOT NULL,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP
);


ALTER TABLE public.windmill_migrations OWNER TO postgres;

--
-- Name: worker_ping; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.worker_ping (
    worker character varying(255) NOT NULL,
    worker_instance character varying(255) NOT NULL,
    ping_at timestamp with time zone DEFAULT now() NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    ip character varying(50) DEFAULT 'NO IP'::character varying NOT NULL,
    jobs_executed integer DEFAULT 0 NOT NULL,
    custom_tags text[],
    worker_group character varying(255) DEFAULT 'default'::character varying NOT NULL,
    dedicated_worker character varying(255),
    wm_version character varying(255) DEFAULT ''::character varying NOT NULL,
    current_job_id uuid,
    current_job_workspace_id character varying(50),
    vcpus bigint,
    memory bigint,
    occupancy_rate real,
    memory_usage bigint,
    wm_memory_usage bigint,
    occupancy_rate_15s real,
    occupancy_rate_5m real,
    occupancy_rate_30m real
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
-- Name: workspace_env; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.workspace_env (
    workspace_id character varying(50) NOT NULL,
    name character varying(255) NOT NULL,
    value character varying(1000) NOT NULL
);


ALTER TABLE public.workspace_env OWNER TO postgres;

--
-- Name: workspace_invite; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.workspace_invite (
    workspace_id character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
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
    ai_config jsonb,
    error_handler_extra_args json,
    error_handler_muted_on_cancel boolean DEFAULT false NOT NULL,
    large_file_storage jsonb,
    git_sync jsonb,
    default_app character varying(255),
    auto_add boolean DEFAULT false,
    default_scripts jsonb,
    deploy_ui jsonb,
    mute_critical_alerts boolean,
    color character varying(7) DEFAULT NULL::character varying,
    operator_settings jsonb DEFAULT '{"runs": true, "groups": true, "folders": true, "workers": true, "triggers": true, "resources": true, "schedules": true, "variables": true, "audit_logs": true}'::jsonb,
    teams_command_script text,
    teams_team_id text,
    teams_team_name text,
    git_app_installations jsonb DEFAULT '[]'::jsonb NOT NULL
);


ALTER TABLE public.workspace_settings OWNER TO postgres;

--
-- Name: zombie_job_counter; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.zombie_job_counter (
    job_id uuid NOT NULL,
    counter integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.zombie_job_counter OWNER TO postgres;

--
-- Name: account id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.account ALTER COLUMN id SET DEFAULT nextval('public.account_id_seq'::regclass);


--
-- Name: alerts id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.alerts ALTER COLUMN id SET DEFAULT nextval('public.alerts_id_seq'::regclass);


--
-- Name: app id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app ALTER COLUMN id SET DEFAULT nextval('public.app_id_seq'::regclass);


--
-- Name: app_script id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_script ALTER COLUMN id SET DEFAULT nextval('public.app_script_id_seq'::regclass);


--
-- Name: app_script app; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_script ALTER COLUMN app SET DEFAULT nextval('public.app_script_app_seq'::regclass);


--
-- Name: app_version id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_version ALTER COLUMN id SET DEFAULT nextval('public.app_version_id_seq'::regclass);


--
-- Name: app_version_lite id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_version_lite ALTER COLUMN id SET DEFAULT nextval('public.app_version_lite_id_seq'::regclass);


--
-- Name: audit id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.audit ALTER COLUMN id SET DEFAULT nextval('public.audit_id_seq'::regclass);


--
-- Name: autoscaling_event id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.autoscaling_event ALTER COLUMN id SET DEFAULT nextval('public.autoscaling_event_id_seq'::regclass);


--
-- Name: flow_node id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_node ALTER COLUMN id SET DEFAULT nextval('public.flow_node_id_seq'::regclass);


--
-- Name: flow_version id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_version ALTER COLUMN id SET DEFAULT nextval('public.flow_version_id_seq'::regclass);


--
-- Name: flow_version_lite id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_version_lite ALTER COLUMN id SET DEFAULT nextval('public.flow_version_lite_id_seq'::regclass);


--
-- Name: healthchecks id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.healthchecks ALTER COLUMN id SET DEFAULT nextval('public.healthchecks_id_seq'::regclass);


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
-- Name: alerts alerts_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.alerts
    ADD CONSTRAINT alerts_pkey PRIMARY KEY (id);


--
-- Name: app app_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app
    ADD CONSTRAINT app_pkey PRIMARY KEY (id);


--
-- Name: app_script app_script_hash_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_script
    ADD CONSTRAINT app_script_hash_key UNIQUE (hash);


--
-- Name: app_script app_script_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_script
    ADD CONSTRAINT app_script_pkey PRIMARY KEY (id);


--
-- Name: app_version_lite app_version_lite_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_version_lite
    ADD CONSTRAINT app_version_lite_pkey PRIMARY KEY (id);


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
-- Name: autoscaling_event autoscaling_event_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.autoscaling_event
    ADD CONSTRAINT autoscaling_event_pkey PRIMARY KEY (id);


--
-- Name: capture_config capture_config_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.capture_config
    ADD CONSTRAINT capture_config_pkey PRIMARY KEY (workspace_id, path, is_flow, trigger_kind);


--
-- Name: capture capture_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.capture
    ADD CONSTRAINT capture_pkey PRIMARY KEY (id);


--
-- Name: cloud_workspace_settings cloud_workspace_settings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.cloud_workspace_settings
    ADD CONSTRAINT cloud_workspace_settings_pkey PRIMARY KEY (workspace_id);


--
-- Name: v2_job_completed completed_job_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job_completed
    ADD CONSTRAINT completed_job_pkey PRIMARY KEY (id);


--
-- Name: concurrency_counter concurrency_counter_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.concurrency_counter
    ADD CONSTRAINT concurrency_counter_pkey PRIMARY KEY (concurrency_id);


--
-- Name: concurrency_key concurrency_key_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.concurrency_key
    ADD CONSTRAINT concurrency_key_pkey PRIMARY KEY (job_id);


--
-- Name: concurrency_locks concurrency_locks_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.concurrency_locks
    ADD CONSTRAINT concurrency_locks_pkey PRIMARY KEY (id);


--
-- Name: custom_concurrency_key_ended custom_concurrency_key_ended_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.custom_concurrency_key_ended
    ADD CONSTRAINT custom_concurrency_key_ended_pkey PRIMARY KEY (key, ended_at);


--
-- Name: dependency_map dependency_map_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_map
    ADD CONSTRAINT dependency_map_pkey PRIMARY KEY (workspace_id, importer_node_id, importer_kind, importer_path, imported_path);


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
-- Name: flow_node flow_node_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_node
    ADD CONSTRAINT flow_node_pkey PRIMARY KEY (id);


--
-- Name: flow_node flow_node_unique_2; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_node
    ADD CONSTRAINT flow_node_unique_2 UNIQUE (path, workspace_id, hash_v2);


--
-- Name: flow flow_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow
    ADD CONSTRAINT flow_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: flow_version_lite flow_version_lite_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_version_lite
    ADD CONSTRAINT flow_version_lite_pkey PRIMARY KEY (id);


--
-- Name: flow_version flow_version_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_version
    ADD CONSTRAINT flow_version_pkey PRIMARY KEY (id);


--
-- Name: folder folder_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.folder
    ADD CONSTRAINT folder_pkey PRIMARY KEY (workspace_id, name);


--
-- Name: gcp_trigger gcp_trigger_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.gcp_trigger
    ADD CONSTRAINT gcp_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: global_settings global_settings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.global_settings
    ADD CONSTRAINT global_settings_pkey PRIMARY KEY (name);


--
-- Name: group_ group__pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_
    ADD CONSTRAINT group__pkey PRIMARY KEY (workspace_id, name);


--
-- Name: healthchecks healthchecks_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.healthchecks
    ADD CONSTRAINT healthchecks_pkey PRIMARY KEY (id);


--
-- Name: http_trigger http_trigger_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.http_trigger
    ADD CONSTRAINT http_trigger_pkey PRIMARY KEY (path, workspace_id);


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
-- Name: job_logs job_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.job_logs
    ADD CONSTRAINT job_logs_pkey PRIMARY KEY (job_id);


--
-- Name: job_perms job_perms_pk; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.job_perms
    ADD CONSTRAINT job_perms_pk PRIMARY KEY (job_id);


--
-- Name: v2_job job_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job
    ADD CONSTRAINT job_pkey PRIMARY KEY (id);


--
-- Name: job_stats job_stats_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.job_stats
    ADD CONSTRAINT job_stats_pkey PRIMARY KEY (workspace_id, job_id, metric_id);


--
-- Name: kafka_trigger kafka_trigger_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.kafka_trigger
    ADD CONSTRAINT kafka_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: log_file log_file_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.log_file
    ADD CONSTRAINT log_file_pkey PRIMARY KEY (hostname, log_ts);


--
-- Name: magic_link magic_link_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.magic_link
    ADD CONSTRAINT magic_link_pkey PRIMARY KEY (email, token);


--
-- Name: mqtt_trigger mqtt_trigger_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.mqtt_trigger
    ADD CONSTRAINT mqtt_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: nats_trigger nats_trigger_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.nats_trigger
    ADD CONSTRAINT nats_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: outstanding_wait_time outstanding_wait_time_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.outstanding_wait_time
    ADD CONSTRAINT outstanding_wait_time_pkey PRIMARY KEY (job_id);


--
-- Name: parallel_monitor_lock parallel_monitor_lock_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.parallel_monitor_lock
    ADD CONSTRAINT parallel_monitor_lock_pkey PRIMARY KEY (parent_flow_id, job_id);


--
-- Name: password password_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.password
    ADD CONSTRAINT password_pkey PRIMARY KEY (email);


--
-- Name: pending_user pending_user_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.pending_user
    ADD CONSTRAINT pending_user_pkey PRIMARY KEY (email);


--
-- Name: pip_resolution_cache pip_resolution_cache_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.pip_resolution_cache
    ADD CONSTRAINT pip_resolution_cache_pkey PRIMARY KEY (hash);


--
-- Name: postgres_trigger pk_postgres_trigger; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.postgres_trigger
    ADD CONSTRAINT pk_postgres_trigger PRIMARY KEY (path, workspace_id);


--
-- Name: sqs_trigger pk_sqs_trigger; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.sqs_trigger
    ADD CONSTRAINT pk_sqs_trigger PRIMARY KEY (path, workspace_id);


--
-- Name: v2_job_queue queue_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job_queue
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
-- Name: tutorial_progress tutorial_progress_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tutorial_progress
    ADD CONSTRAINT tutorial_progress_pkey PRIMARY KEY (email);


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
-- Name: v2_job_runtime v2_job_runtime_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job_runtime
    ADD CONSTRAINT v2_job_runtime_pkey PRIMARY KEY (id);


--
-- Name: v2_job_status v2_job_status_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job_status
    ADD CONSTRAINT v2_job_status_pkey PRIMARY KEY (id);


--
-- Name: variable variable_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.variable
    ADD CONSTRAINT variable_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: websocket_trigger websocket_trigger_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.websocket_trigger
    ADD CONSTRAINT websocket_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: windmill_migrations windmill_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.windmill_migrations
    ADD CONSTRAINT windmill_migrations_pkey PRIMARY KEY (name);


--
-- Name: config worker_group_config_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.config
    ADD CONSTRAINT worker_group_config_pkey PRIMARY KEY (name);


--
-- Name: worker_ping worker_ping_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.worker_ping
    ADD CONSTRAINT worker_ping_pkey PRIMARY KEY (worker);


--
-- Name: workspace_env workspace_env_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_env
    ADD CONSTRAINT workspace_env_pkey PRIMARY KEY (workspace_id, name);


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
-- Name: zombie_job_counter zombie_job_counter_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.zombie_job_counter
    ADD CONSTRAINT zombie_job_counter_pkey PRIMARY KEY (job_id);


--
-- Name: alerts_by_workspace; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX alerts_by_workspace ON public.alerts USING btree (workspace_id);


--
-- Name: app_workspace_with_hash_unique_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX app_workspace_with_hash_unique_idx ON public.workspace_runnable_dependencies USING btree (app_path, runnable_path, script_hash, runnable_is_flow, workspace_id) WHERE (script_hash IS NOT NULL);


--
-- Name: app_workspace_without_hash_unique_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX app_workspace_without_hash_unique_idx ON public.workspace_runnable_dependencies USING btree (app_path, runnable_path, runnable_is_flow, workspace_id) WHERE (script_hash IS NULL);


--
-- Name: autoscaling_event_worker_group_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX autoscaling_event_worker_group_idx ON public.autoscaling_event USING btree (worker_group, applied_at);


--
-- Name: concurrency_key_ended_at_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX concurrency_key_ended_at_idx ON public.concurrency_key USING btree (key, ended_at DESC);


--
-- Name: dependency_map_imported_path_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX dependency_map_imported_path_idx ON public.dependency_map USING btree (workspace_id, imported_path);


--
-- Name: deployment_metadata_app; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX deployment_metadata_app ON public.deployment_metadata USING btree (workspace_id, path, app_version) WHERE (app_version IS NOT NULL);


--
-- Name: deployment_metadata_flow; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX deployment_metadata_flow ON public.deployment_metadata USING btree (workspace_id, path, flow_version) WHERE (flow_version IS NOT NULL);


--
-- Name: deployment_metadata_script; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX deployment_metadata_script ON public.deployment_metadata USING btree (workspace_id, script_hash) WHERE (script_hash IS NOT NULL);


--
-- Name: flow_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX flow_extra_perms ON public.flow USING gin (extra_perms);


--
-- Name: flow_node_hash; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX flow_node_hash ON public.flow_node USING btree (hash);


--
-- Name: flow_workspace_runnable_path_is_flow_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX flow_workspace_runnable_path_is_flow_idx ON public.workspace_runnable_dependencies USING btree (runnable_path, runnable_is_flow, workspace_id);


--
-- Name: flow_workspace_with_hash_unique_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX flow_workspace_with_hash_unique_idx ON public.workspace_runnable_dependencies USING btree (flow_path, runnable_path, script_hash, runnable_is_flow, workspace_id) WHERE (script_hash IS NOT NULL);


--
-- Name: flow_workspace_without_hash_unique_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX flow_workspace_without_hash_unique_idx ON public.workspace_runnable_dependencies USING btree (flow_path, runnable_path, runnable_is_flow, workspace_id) WHERE (script_hash IS NULL);


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
-- Name: healthchecks_check_type_created_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX healthchecks_check_type_created_at ON public.healthchecks USING btree (check_type, created_at);


--
-- Name: idx_metrics_id_created_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_metrics_id_created_at ON public.metrics USING btree (id, created_at DESC) WHERE ((id)::text ~~ 'queue_%'::text);


--
-- Name: index_flow_version_path_created_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_flow_version_path_created_at ON public.flow_version USING btree (path, created_at);


--
-- Name: index_magic_link_exp; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_magic_link_exp ON public.magic_link USING btree (expiration);


--
-- Name: index_script_on_path_created_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_script_on_path_created_at ON public.script USING btree (workspace_id, path, created_at DESC);


--
-- Name: index_token_exp; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_token_exp ON public.token USING btree (expiration);


--
-- Name: index_usr_email; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_usr_email ON public.usr USING btree (email);


--
-- Name: ix_audit_timestamps; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_audit_timestamps ON public.audit USING btree ("timestamp" DESC);


--
-- Name: ix_completed_job_workspace_id_started_at_new_2; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_completed_job_workspace_id_started_at_new_2 ON public.v2_job_completed USING btree (workspace_id, started_at DESC);


--
-- Name: ix_job_completed_completed_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX ix_job_completed_completed_at ON public.v2_job_completed USING btree (completed_at DESC);


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
-- Name: job_stats_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX job_stats_id ON public.job_stats USING btree (job_id);


--
-- Name: labeled_jobs_on_jobs; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX labeled_jobs_on_jobs ON public.v2_job_completed USING gin (((result -> 'wm_labels'::text))) WHERE (result ? 'wm_labels'::text);


--
-- Name: log_file_log_ts_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX log_file_log_ts_idx ON public.log_file USING btree (log_ts);


--
-- Name: metrics_key_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX metrics_key_idx ON public.metrics USING btree (id);


--
-- Name: metrics_sort_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX metrics_sort_idx ON public.metrics USING btree (created_at DESC);


--
-- Name: queue_sort_v2; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX queue_sort_v2 ON public.v2_job_queue USING btree (priority DESC NULLS LAST, scheduled_for, tag) WHERE (running = false);


--
-- Name: queue_suspended; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX queue_suspended ON public.v2_job_queue USING btree (priority DESC NULLS LAST, created_at, suspend_until, suspend, tag) WHERE (suspend_until IS NOT NULL);


--
-- Name: resource_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX resource_extra_perms ON public.resource USING gin (extra_perms);


--
-- Name: root_queue_index_by_path; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX root_queue_index_by_path ON public.v2_job_queue USING btree (workspace_id, created_at);


--
-- Name: schedule_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX schedule_extra_perms ON public.schedule USING gin (extra_perms);


--
-- Name: script_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX script_extra_perms ON public.script USING gin (extra_perms);


--
-- Name: unique_subscription_per_gcp_resource; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX unique_subscription_per_gcp_resource ON public.gcp_trigger USING btree (subscription_id, gcp_resource_path, workspace_id);


--
-- Name: v2_job_queue_suspend; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX v2_job_queue_suspend ON public.v2_job_queue USING btree (workspace_id, suspend) WHERE (suspend > 0);


--
-- Name: variable_extra_perms; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX variable_extra_perms ON public.variable USING gin (extra_perms);


--
-- Name: worker_ping_on_ping_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX worker_ping_on_ping_at ON public.worker_ping USING btree (ping_at);


--
-- Name: http_trigger check_route_path_change; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER check_route_path_change BEFORE UPDATE ON public.http_trigger FOR EACH ROW EXECUTE FUNCTION public.prevent_route_path_change();


--
-- Name: flow_version flow_update_trigger; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER flow_update_trigger AFTER INSERT ON public.flow_version FOR EACH ROW EXECUTE FUNCTION public.notify_runnable_version_change('flow');


--
-- Name: http_trigger http_trigger_change_trigger; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER http_trigger_change_trigger AFTER INSERT OR DELETE OR UPDATE ON public.http_trigger FOR EACH ROW EXECUTE FUNCTION public.notify_http_trigger_change();


--
-- Name: config notify_config_change; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER notify_config_change AFTER INSERT OR UPDATE ON public.config FOR EACH ROW EXECUTE FUNCTION public.notify_config_change();


--
-- Name: global_settings notify_global_setting_change; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER notify_global_setting_change AFTER INSERT OR UPDATE ON public.global_settings FOR EACH ROW EXECUTE FUNCTION public.notify_global_setting_change();


--
-- Name: global_settings notify_global_setting_delete; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER notify_global_setting_delete AFTER DELETE ON public.global_settings FOR EACH ROW EXECUTE FUNCTION public.notify_global_setting_delete();


--
-- Name: script script_update_trigger; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER script_update_trigger AFTER UPDATE OF lock ON public.script FOR EACH ROW EXECUTE FUNCTION public.notify_runnable_version_change('script');


--
-- Name: token token_invalidation_trigger; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER token_invalidation_trigger AFTER DELETE ON public.token FOR EACH ROW EXECUTE FUNCTION public.notify_token_invalidation();


--
-- Name: workspace_settings webhook_change_trigger; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER webhook_change_trigger AFTER UPDATE OF webhook ON public.workspace_settings FOR EACH ROW WHEN ((old.webhook IS DISTINCT FROM new.webhook)) EXECUTE FUNCTION public.notify_webhook_change();


--
-- Name: workspace_env workspace_envs_change_trigger; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER workspace_envs_change_trigger AFTER INSERT OR DELETE OR UPDATE OF name, value ON public.workspace_env FOR EACH ROW EXECUTE FUNCTION public.notify_workspace_envs_change();


--
-- Name: workspace workspace_premium_change_trigger; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER workspace_premium_change_trigger AFTER UPDATE OF premium ON public.workspace FOR EACH ROW EXECUTE FUNCTION public.notify_workspace_premium_change();


--
-- Name: account account_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.account
    ADD CONSTRAINT account_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: app_script app_script_app_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_script
    ADD CONSTRAINT app_script_app_fkey FOREIGN KEY (app) REFERENCES public.app(id) ON DELETE CASCADE;


--
-- Name: app_version app_version_flow_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_version
    ADD CONSTRAINT app_version_flow_id_fkey FOREIGN KEY (app_id) REFERENCES public.app(id) ON DELETE CASCADE;


--
-- Name: app_version_lite app_version_lite_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app_version_lite
    ADD CONSTRAINT app_version_lite_id_fkey FOREIGN KEY (id) REFERENCES public.app_version(id) ON DELETE CASCADE;


--
-- Name: app app_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.app
    ADD CONSTRAINT app_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: capture_config capture_config_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.capture_config
    ADD CONSTRAINT capture_config_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id) ON DELETE CASCADE;


--
-- Name: capture capture_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.capture
    ADD CONSTRAINT capture_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: cloud_workspace_settings cloud_workspace_settings_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.cloud_workspace_settings
    ADD CONSTRAINT cloud_workspace_settings_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id) ON DELETE CASCADE;


--
-- Name: deployment_metadata deployment_metadata_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.deployment_metadata
    ADD CONSTRAINT deployment_metadata_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


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
-- Name: postgres_trigger fk_postgres_trigger_workspace; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.postgres_trigger
    ADD CONSTRAINT fk_postgres_trigger_workspace FOREIGN KEY (workspace_id) REFERENCES public.workspace(id) ON DELETE CASCADE;


--
-- Name: sqs_trigger fk_sqs_trigger_workspace; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.sqs_trigger
    ADD CONSTRAINT fk_sqs_trigger_workspace FOREIGN KEY (workspace_id) REFERENCES public.workspace(id) ON DELETE CASCADE;


--
-- Name: workspace_runnable_dependencies fk_workspace_runnable_dependencies_app_path; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_runnable_dependencies
    ADD CONSTRAINT fk_workspace_runnable_dependencies_app_path FOREIGN KEY (app_path, workspace_id) REFERENCES public.app(path, workspace_id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- Name: flow_node flow_node_path_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_node
    ADD CONSTRAINT flow_node_path_workspace_id_fkey FOREIGN KEY (path, workspace_id) REFERENCES public.flow(path, workspace_id) ON DELETE CASCADE;


--
-- Name: flow_node flow_node_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_node
    ADD CONSTRAINT flow_node_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: flow_version_lite flow_version_lite_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_version_lite
    ADD CONSTRAINT flow_version_lite_id_fkey FOREIGN KEY (id) REFERENCES public.flow_version(id) ON DELETE CASCADE;


--
-- Name: flow_version flow_version_workspace_id_path_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow_version
    ADD CONSTRAINT flow_version_workspace_id_path_fkey FOREIGN KEY (workspace_id, path) REFERENCES public.flow(workspace_id, path) ON DELETE CASCADE;


--
-- Name: flow flow_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.flow
    ADD CONSTRAINT flow_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: workspace_runnable_dependencies flow_workspace_runnables_workspace_id_flow_path_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.workspace_runnable_dependencies
    ADD CONSTRAINT flow_workspace_runnables_workspace_id_flow_path_fkey FOREIGN KEY (flow_path, workspace_id) REFERENCES public.flow(path, workspace_id) ON UPDATE CASCADE ON DELETE CASCADE;


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
-- Name: job_stats job_stats_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.job_stats
    ADD CONSTRAINT job_stats_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id);


--
-- Name: nats_trigger nats_trigger_workspace_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.nats_trigger
    ADD CONSTRAINT nats_trigger_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES public.workspace(id) ON DELETE CASCADE;


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
    ADD CONSTRAINT resume_job_flow_fkey FOREIGN KEY (flow) REFERENCES public.v2_job_queue(id) ON DELETE CASCADE;


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
-- Name: v2_job_runtime v2_job_runtime_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job_runtime
    ADD CONSTRAINT v2_job_runtime_id_fkey FOREIGN KEY (id) REFERENCES public.v2_job_queue(id) ON DELETE CASCADE;


--
-- Name: v2_job_status v2_job_status_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job_status
    ADD CONSTRAINT v2_job_status_id_fkey FOREIGN KEY (id) REFERENCES public.v2_job_queue(id) ON DELETE CASCADE;


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
-- Name: zombie_job_counter zombie_job_counter_job_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.zombie_job_counter
    ADD CONSTRAINT zombie_job_counter_job_id_fkey FOREIGN KEY (job_id) REFERENCES public.v2_job(id) ON DELETE CASCADE;


--
-- Name: account admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.account TO windmill_admin USING (true);


--
-- Name: app admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.app TO windmill_admin USING (true);


--
-- Name: audit admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.audit TO windmill_admin USING (true);


--
-- Name: capture admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.capture TO windmill_admin USING (true);


--
-- Name: capture_config admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.capture_config TO windmill_admin USING (true);


--
-- Name: flow admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.flow TO windmill_admin USING (true);


--
-- Name: folder admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.folder TO windmill_admin USING (true);


--
-- Name: gcp_trigger admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.gcp_trigger TO windmill_admin USING (true);


--
-- Name: http_trigger admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.http_trigger TO windmill_admin USING (true);


--
-- Name: kafka_trigger admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.kafka_trigger TO windmill_admin USING (true);


--
-- Name: mqtt_trigger admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.mqtt_trigger TO windmill_admin USING (true);


--
-- Name: nats_trigger admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.nats_trigger TO windmill_admin USING (true);


--
-- Name: postgres_trigger admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.postgres_trigger TO windmill_admin USING (true);


--
-- Name: raw_app admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.raw_app TO windmill_admin USING (true);


--
-- Name: resource admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.resource TO windmill_admin USING (true);


--
-- Name: schedule admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.schedule TO windmill_admin USING (true);


--
-- Name: script admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.script TO windmill_admin USING (true);


--
-- Name: sqs_trigger admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.sqs_trigger TO windmill_admin USING (true);


--
-- Name: usr_to_group admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.usr_to_group TO windmill_admin USING (true);


--
-- Name: v2_job admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.v2_job TO windmill_admin USING (true);


--
-- Name: v2_job_completed admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.v2_job_completed TO windmill_admin USING (true);


--
-- Name: v2_job_queue admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.v2_job_queue TO windmill_admin USING (true);


--
-- Name: v2_job_runtime admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.v2_job_runtime TO windmill_admin;


--
-- Name: v2_job_status admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.v2_job_status TO windmill_admin;


--
-- Name: variable admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.variable TO windmill_admin USING (true);


--
-- Name: websocket_trigger admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY admin_policy ON public.websocket_trigger TO windmill_admin USING (true);


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
-- Name: capture_config; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.capture_config ENABLE ROW LEVEL SECURITY;

--
-- Name: flow; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.flow ENABLE ROW LEVEL SECURITY;

--
-- Name: folder; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.folder ENABLE ROW LEVEL SECURITY;

--
-- Name: gcp_trigger; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.gcp_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: http_trigger; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.http_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: kafka_trigger; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.kafka_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: mqtt_trigger; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.mqtt_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: nats_trigger; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.nats_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: postgres_trigger; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.postgres_trigger ENABLE ROW LEVEL SECURITY;

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

CREATE POLICY schedule ON public.audit FOR INSERT TO windmill_user WITH CHECK (((username)::text ~~ 'schedule-%'::text));


--
-- Name: schedule; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.schedule ENABLE ROW LEVEL SECURITY;

--
-- Name: audit schedule_audit; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY schedule_audit ON public.audit FOR INSERT TO windmill_user WITH CHECK (((parameters ->> 'end_user'::text) ~~ 'schedule-%'::text));


--
-- Name: script; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.script ENABLE ROW LEVEL SECURITY;

--
-- Name: usr_to_group see_extra_perms_groups; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups ON public.usr_to_group TO windmill_user USING (true) WITH CHECK ((EXISTS ( SELECT f.key,
    f.value
   FROM public.group_ g,
    LATERAL jsonb_each_text(g.extra_perms) f(key, value)
  WHERE (((usr_to_group.group_)::text = (g.name)::text) AND ((usr_to_group.workspace_id)::text = (g.workspace_id)::text) AND (split_part(f.key, '/'::text, 1) = 'g'::text) AND (f.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (f.value)::boolean))));


--
-- Name: app see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.app FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: flow see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.flow FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(flow.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: folder see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.folder FOR DELETE TO windmill_user USING ((EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))))));


--
-- Name: gcp_trigger see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.gcp_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(gcp_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: kafka_trigger see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.kafka_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(kafka_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: mqtt_trigger see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.mqtt_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(mqtt_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: nats_trigger see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.nats_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(nats_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: postgres_trigger see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.postgres_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(postgres_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: raw_app see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.raw_app FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(raw_app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: resource see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.resource FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(resource.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: schedule see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.schedule FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(schedule.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: script see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.script FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(script.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: sqs_trigger see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.sqs_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(sqs_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: variable see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.variable FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(variable.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: websocket_trigger see_extra_perms_groups_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_delete ON public.websocket_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(websocket_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: app see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.app FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: flow see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.flow FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(flow.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: folder see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.folder FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))))));


--
-- Name: gcp_trigger see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.gcp_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(gcp_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: kafka_trigger see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.kafka_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(kafka_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: mqtt_trigger see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.mqtt_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(mqtt_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: nats_trigger see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.nats_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(nats_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: postgres_trigger see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.postgres_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(postgres_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: raw_app see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.raw_app FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(raw_app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: resource see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.resource FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(resource.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: schedule see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.schedule FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(schedule.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: script see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.script FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(script.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: sqs_trigger see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.sqs_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(sqs_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: variable see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.variable FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(variable.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: websocket_trigger see_extra_perms_groups_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_insert ON public.websocket_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(websocket_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: app see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.app FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: flow see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.flow FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: folder see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.folder FOR SELECT TO windmill_user USING (((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)) OR (EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)))))));


--
-- Name: gcp_trigger see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.gcp_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: http_trigger see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.http_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: kafka_trigger see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.kafka_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: mqtt_trigger see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.mqtt_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: nats_trigger see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.nats_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: postgres_trigger see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.postgres_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: raw_app see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.raw_app FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: resource see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.resource FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: schedule see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.schedule FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: script see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.script FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: sqs_trigger see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.sqs_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: variable see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.variable FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: websocket_trigger see_extra_perms_groups_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_select ON public.websocket_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: app see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.app FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: flow see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.flow FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(flow.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: folder see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.folder FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))))));


--
-- Name: gcp_trigger see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.gcp_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(gcp_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: http_trigger see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.http_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(http_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: kafka_trigger see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.kafka_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(kafka_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: mqtt_trigger see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.mqtt_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(mqtt_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: nats_trigger see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.nats_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(nats_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: postgres_trigger see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.postgres_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(postgres_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: raw_app see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.raw_app FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(raw_app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: resource see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.resource FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(resource.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: schedule see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.schedule FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(schedule.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: script see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.script FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(script.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: sqs_trigger see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.sqs_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(sqs_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: variable see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.variable FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(variable.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: websocket_trigger see_extra_perms_groups_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_groups_update ON public.websocket_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(websocket_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: usr_to_group see_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user ON public.usr_to_group TO windmill_user USING (true) WITH CHECK ((EXISTS ( SELECT 1
   FROM public.group_
  WHERE (((usr_to_group.group_)::text = (group_.name)::text) AND ((usr_to_group.workspace_id)::text = (group_.workspace_id)::text) AND ((group_.extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean))));


--
-- Name: app see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.app FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: flow see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.flow FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: folder see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.folder FOR DELETE TO windmill_user USING ((concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[])));


--
-- Name: gcp_trigger see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.gcp_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: kafka_trigger see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.kafka_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: mqtt_trigger see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.mqtt_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: nats_trigger see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.nats_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: postgres_trigger see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.postgres_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: raw_app see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.raw_app FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: resource see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.resource FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: schedule see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.schedule FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: script see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.script FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: sqs_trigger see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.sqs_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: variable see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.variable FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: websocket_trigger see_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_delete ON public.websocket_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: app see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.app FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: flow see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.flow FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: folder see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.folder FOR INSERT TO windmill_user WITH CHECK ((concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[])));


--
-- Name: gcp_trigger see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.gcp_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: kafka_trigger see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.kafka_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: mqtt_trigger see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.mqtt_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: nats_trigger see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.nats_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: postgres_trigger see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.postgres_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: raw_app see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.raw_app FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: resource see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.resource FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: schedule see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.schedule FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: script see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.script FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: sqs_trigger see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.sqs_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: variable see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.variable FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: websocket_trigger see_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_insert ON public.websocket_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: app see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.app FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: flow see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.flow FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: folder see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.folder FOR SELECT TO windmill_user USING (((extra_perms ? concat('u/', current_setting('session.user'::text))) OR (concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[]))));


--
-- Name: gcp_trigger see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.gcp_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: http_trigger see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.http_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: kafka_trigger see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.kafka_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: mqtt_trigger see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.mqtt_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: nats_trigger see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.nats_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: postgres_trigger see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.postgres_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: raw_app see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.raw_app FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: resource see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.resource FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: schedule see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.schedule FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: script see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.script FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: sqs_trigger see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.sqs_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: variable see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.variable FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: websocket_trigger see_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_select ON public.websocket_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: app see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.app FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: flow see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.flow FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: folder see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.folder FOR UPDATE TO windmill_user USING ((concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[])));


--
-- Name: gcp_trigger see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.gcp_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: http_trigger see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.http_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: kafka_trigger see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.kafka_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: mqtt_trigger see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.mqtt_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: nats_trigger see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.nats_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: postgres_trigger see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.postgres_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: raw_app see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.raw_app FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: resource see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.resource FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: schedule see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.schedule FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: script see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.script FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: sqs_trigger see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.sqs_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: variable see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.variable FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: websocket_trigger see_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_extra_perms_user_update ON public.websocket_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: v2_job see_folder_extra_perms_user; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user ON public.v2_job TO windmill_user USING (((visible_to_owner IS TRUE) AND (split_part((runnable_path)::text, '/'::text, 1) = 'f'::text) AND (split_part((runnable_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: app see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.app FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.capture FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture_config see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.capture_config FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: flow see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.flow FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: gcp_trigger see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.gcp_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: kafka_trigger see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.kafka_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: mqtt_trigger see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.mqtt_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: nats_trigger see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.nats_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: postgres_trigger see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.postgres_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: raw_app see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.raw_app FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: resource see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.resource FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: schedule see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.schedule FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: script see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.script FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: sqs_trigger see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.sqs_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: variable see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.variable FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: websocket_trigger see_folder_extra_perms_user_delete; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_delete ON public.websocket_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: app see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.app FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.capture FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture_config see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.capture_config FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: flow see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.flow FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: gcp_trigger see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.gcp_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: kafka_trigger see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.kafka_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: mqtt_trigger see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.mqtt_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: nats_trigger see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.nats_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: postgres_trigger see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.postgres_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: raw_app see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.raw_app FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: resource see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.resource FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: schedule see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.schedule FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: script see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.script FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: sqs_trigger see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.sqs_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: variable see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.variable FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: websocket_trigger see_folder_extra_perms_user_insert; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_insert ON public.websocket_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: app see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.app FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: capture see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.capture FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: capture_config see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.capture_config FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: flow see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.flow FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: gcp_trigger see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.gcp_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: http_trigger see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.http_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: kafka_trigger see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.kafka_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: mqtt_trigger see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.mqtt_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: nats_trigger see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.nats_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: postgres_trigger see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.postgres_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: raw_app see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.raw_app FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: resource see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.resource FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: schedule see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.schedule FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: script see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.script FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: sqs_trigger see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.sqs_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: variable see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.variable FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: websocket_trigger see_folder_extra_perms_user_select; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_select ON public.websocket_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: app see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.app FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.capture FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture_config see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.capture_config FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: flow see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.flow FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: gcp_trigger see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.gcp_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: http_trigger see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.http_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: kafka_trigger see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.kafka_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: mqtt_trigger see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.mqtt_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: nats_trigger see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.nats_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: postgres_trigger see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.postgres_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: raw_app see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.raw_app FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: resource see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.resource FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: schedule see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.schedule FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: script see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.script FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: sqs_trigger see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.sqs_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: variable see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.variable FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: websocket_trigger see_folder_extra_perms_user_update; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_folder_extra_perms_user_update ON public.websocket_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture see_from_allowed_runnables; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_from_allowed_runnables ON public.capture TO windmill_user USING (((is_flow AND (EXISTS ( SELECT 1
   FROM public.flow
  WHERE (((flow.workspace_id)::text = (capture.workspace_id)::text) AND ((flow.path)::text = (capture.path)::text))))) OR ((NOT is_flow) AND (EXISTS ( SELECT 1
   FROM public.script
  WHERE (((script.workspace_id)::text = (capture.workspace_id)::text) AND ((script.path)::text = (capture.path)::text)))))));


--
-- Name: capture_config see_from_allowed_runnables; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_from_allowed_runnables ON public.capture_config TO windmill_user USING (((is_flow AND (EXISTS ( SELECT 1
   FROM public.flow
  WHERE (((flow.workspace_id)::text = (capture_config.workspace_id)::text) AND ((flow.path)::text = (capture_config.path)::text))))) OR ((NOT is_flow) AND (EXISTS ( SELECT 1
   FROM public.script
  WHERE (((script.workspace_id)::text = (capture_config.workspace_id)::text) AND ((script.path)::text = (capture_config.path)::text)))))));


--
-- Name: app see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.app TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: capture see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.capture TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: capture_config see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.capture_config TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: flow see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.flow TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: gcp_trigger see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.gcp_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: kafka_trigger see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.kafka_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: mqtt_trigger see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.mqtt_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: nats_trigger see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.nats_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: postgres_trigger see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.postgres_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: raw_app see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.raw_app TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: resource see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.resource TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: schedule see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.schedule TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: script see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.script TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: sqs_trigger see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.sqs_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: v2_job see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.v2_job TO windmill_user USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'g'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: variable see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.variable TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: websocket_trigger see_member; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member ON public.websocket_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: v2_job see_member_path; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_member_path ON public.v2_job TO windmill_user USING (((visible_to_owner IS TRUE) AND (split_part((runnable_path)::text, '/'::text, 1) = 'g'::text) AND (split_part((runnable_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: app see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.app TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: audit see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.audit TO windmill_user USING (((username)::text = current_setting('session.user'::text)));


--
-- Name: capture see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.capture TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: capture_config see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.capture_config TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: flow see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.flow TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: gcp_trigger see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.gcp_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: kafka_trigger see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.kafka_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: mqtt_trigger see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.mqtt_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: nats_trigger see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.nats_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: postgres_trigger see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.postgres_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: raw_app see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.raw_app TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: resource see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.resource TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: schedule see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.schedule TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: script see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.script TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: sqs_trigger see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.sqs_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: v2_job see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.v2_job TO windmill_user USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'u'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: variable see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.variable TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: websocket_trigger see_own; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own ON public.websocket_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: v2_job see_own_path; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY see_own_path ON public.v2_job TO windmill_user USING (((visible_to_owner IS TRUE) AND (split_part((runnable_path)::text, '/'::text, 1) = 'u'::text) AND (split_part((runnable_path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: sqs_trigger; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.sqs_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: usr_to_group; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.usr_to_group ENABLE ROW LEVEL SECURITY;

--
-- Name: v2_job; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.v2_job ENABLE ROW LEVEL SECURITY;

--
-- Name: variable; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.variable ENABLE ROW LEVEL SECURITY;

--
-- Name: audit webhook; Type: POLICY; Schema: public; Owner: postgres
--

CREATE POLICY webhook ON public.audit FOR INSERT TO windmill_user WITH CHECK (((username)::text ~~ 'webhook-%'::text));


--
-- Name: websocket_trigger; Type: ROW SECURITY; Schema: public; Owner: postgres
--

ALTER TABLE public.websocket_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: pg_database_owner
--

GRANT USAGE ON SCHEMA public TO windmill_user;
GRANT USAGE ON SCHEMA public TO windmill_admin;


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
-- Name: TABLE alerts; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.alerts TO windmill_user;
GRANT ALL ON TABLE public.alerts TO windmill_admin;


--
-- Name: SEQUENCE alerts_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.alerts_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.alerts_id_seq TO windmill_admin;


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
-- Name: TABLE app_script; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.app_script TO windmill_user;
GRANT ALL ON TABLE public.app_script TO windmill_admin;


--
-- Name: SEQUENCE app_script_app_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.app_script_app_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.app_script_app_seq TO windmill_admin;


--
-- Name: SEQUENCE app_script_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.app_script_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.app_script_id_seq TO windmill_admin;


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
-- Name: TABLE app_version_lite; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.app_version_lite TO windmill_user;
GRANT ALL ON TABLE public.app_version_lite TO windmill_admin;


--
-- Name: SEQUENCE app_version_lite_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.app_version_lite_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.app_version_lite_id_seq TO windmill_admin;


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
-- Name: TABLE autoscaling_event; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.autoscaling_event TO windmill_user;
GRANT ALL ON TABLE public.autoscaling_event TO windmill_admin;


--
-- Name: SEQUENCE autoscaling_event_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.autoscaling_event_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.autoscaling_event_id_seq TO windmill_admin;


--
-- Name: TABLE capture; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.capture TO windmill_user;
GRANT ALL ON TABLE public.capture TO windmill_admin;


--
-- Name: TABLE capture_config; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.capture_config TO windmill_user;
GRANT ALL ON TABLE public.capture_config TO windmill_admin;


--
-- Name: SEQUENCE capture_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.capture_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.capture_id_seq TO windmill_admin;


--
-- Name: TABLE cloud_workspace_settings; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.cloud_workspace_settings TO windmill_user;
GRANT ALL ON TABLE public.cloud_workspace_settings TO windmill_admin;


--
-- Name: TABLE concurrency_counter; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.concurrency_counter TO windmill_user;
GRANT ALL ON TABLE public.concurrency_counter TO windmill_admin;


--
-- Name: TABLE concurrency_key; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.concurrency_key TO windmill_user;
GRANT ALL ON TABLE public.concurrency_key TO windmill_admin;


--
-- Name: TABLE concurrency_locks; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.concurrency_locks TO windmill_user;
GRANT ALL ON TABLE public.concurrency_locks TO windmill_admin;


--
-- Name: TABLE config; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.config TO windmill_user;
GRANT ALL ON TABLE public.config TO windmill_admin;


--
-- Name: TABLE custom_concurrency_key_ended; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.custom_concurrency_key_ended TO windmill_user;
GRANT ALL ON TABLE public.custom_concurrency_key_ended TO windmill_admin;


--
-- Name: TABLE dependency_map; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.dependency_map TO windmill_user;
GRANT ALL ON TABLE public.dependency_map TO windmill_admin;


--
-- Name: TABLE deployment_metadata; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.deployment_metadata TO windmill_user;
GRANT ALL ON TABLE public.deployment_metadata TO windmill_admin;


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
-- Name: SEQUENCE flow_node_hash_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.flow_node_hash_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.flow_node_hash_seq TO windmill_admin;


--
-- Name: TABLE flow_node; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.flow_node TO windmill_user;
GRANT ALL ON TABLE public.flow_node TO windmill_admin;


--
-- Name: SEQUENCE flow_node_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.flow_node_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.flow_node_id_seq TO windmill_admin;


--
-- Name: TABLE flow_version; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.flow_version TO windmill_user;
GRANT ALL ON TABLE public.flow_version TO windmill_admin;


--
-- Name: SEQUENCE flow_version_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.flow_version_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.flow_version_id_seq TO windmill_admin;


--
-- Name: TABLE flow_version_lite; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.flow_version_lite TO windmill_user;
GRANT ALL ON TABLE public.flow_version_lite TO windmill_admin;


--
-- Name: SEQUENCE flow_version_lite_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.flow_version_lite_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.flow_version_lite_id_seq TO windmill_admin;


--
-- Name: TABLE workspace_runnable_dependencies; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.workspace_runnable_dependencies TO windmill_user;
GRANT ALL ON TABLE public.workspace_runnable_dependencies TO windmill_admin;


--
-- Name: TABLE flow_workspace_runnables; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.flow_workspace_runnables TO windmill_user;
GRANT ALL ON TABLE public.flow_workspace_runnables TO windmill_admin;


--
-- Name: TABLE folder; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.folder TO windmill_user;
GRANT ALL ON TABLE public.folder TO windmill_admin;


--
-- Name: TABLE gcp_trigger; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.gcp_trigger TO windmill_user;
GRANT ALL ON TABLE public.gcp_trigger TO windmill_admin;


--
-- Name: TABLE global_settings; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.global_settings TO windmill_user;
GRANT ALL ON TABLE public.global_settings TO windmill_admin;


--
-- Name: TABLE group_; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.group_ TO windmill_user;
GRANT ALL ON TABLE public.group_ TO windmill_admin;


--
-- Name: TABLE healthchecks; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.healthchecks TO windmill_user;
GRANT ALL ON TABLE public.healthchecks TO windmill_admin;


--
-- Name: SEQUENCE healthchecks_id_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.healthchecks_id_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.healthchecks_id_seq TO windmill_admin;


--
-- Name: TABLE http_trigger; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.http_trigger TO windmill_user;
GRANT ALL ON TABLE public.http_trigger TO windmill_admin;


--
-- Name: SEQUENCE http_trigger_version_seq; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON SEQUENCE public.http_trigger_version_seq TO windmill_user;
GRANT ALL ON SEQUENCE public.http_trigger_version_seq TO windmill_admin;


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
-- Name: TABLE job_logs; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.job_logs TO windmill_user;
GRANT ALL ON TABLE public.job_logs TO windmill_admin;


--
-- Name: TABLE job_perms; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.job_perms TO windmill_user;
GRANT ALL ON TABLE public.job_perms TO windmill_admin;


--
-- Name: TABLE job_stats; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.job_stats TO windmill_user;
GRANT ALL ON TABLE public.job_stats TO windmill_admin;


--
-- Name: TABLE kafka_trigger; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.kafka_trigger TO windmill_user;
GRANT ALL ON TABLE public.kafka_trigger TO windmill_admin;


--
-- Name: TABLE log_file; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.log_file TO windmill_user;
GRANT ALL ON TABLE public.log_file TO windmill_admin;


--
-- Name: TABLE magic_link; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.magic_link TO windmill_user;
GRANT ALL ON TABLE public.magic_link TO windmill_admin;


--
-- Name: TABLE metrics; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.metrics TO windmill_user;
GRANT ALL ON TABLE public.metrics TO windmill_admin;


--
-- Name: TABLE mqtt_trigger; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.mqtt_trigger TO windmill_user;
GRANT ALL ON TABLE public.mqtt_trigger TO windmill_admin;


--
-- Name: TABLE nats_trigger; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.nats_trigger TO windmill_user;
GRANT ALL ON TABLE public.nats_trigger TO windmill_admin;


--
-- Name: TABLE outstanding_wait_time; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.outstanding_wait_time TO windmill_user;
GRANT ALL ON TABLE public.outstanding_wait_time TO windmill_admin;


--
-- Name: TABLE parallel_monitor_lock; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.parallel_monitor_lock TO windmill_user;
GRANT ALL ON TABLE public.parallel_monitor_lock TO windmill_admin;


--
-- Name: TABLE password; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.password TO windmill_user;
GRANT ALL ON TABLE public.password TO windmill_admin;


--
-- Name: TABLE pending_user; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.pending_user TO windmill_user;
GRANT ALL ON TABLE public.pending_user TO windmill_admin;


--
-- Name: TABLE pip_resolution_cache; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.pip_resolution_cache TO windmill_user;
GRANT ALL ON TABLE public.pip_resolution_cache TO windmill_admin;


--
-- Name: TABLE postgres_trigger; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.postgres_trigger TO windmill_user;
GRANT ALL ON TABLE public.postgres_trigger TO windmill_admin;


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
-- Name: TABLE sqs_trigger; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.sqs_trigger TO windmill_user;
GRANT ALL ON TABLE public.sqs_trigger TO windmill_admin;


--
-- Name: TABLE token; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.token TO windmill_user;
GRANT ALL ON TABLE public.token TO windmill_admin;


--
-- Name: TABLE tutorial_progress; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.tutorial_progress TO windmill_user;
GRANT ALL ON TABLE public.tutorial_progress TO windmill_admin;


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
-- Name: TABLE v2_job; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.v2_job TO windmill_user;
GRANT ALL ON TABLE public.v2_job TO windmill_admin;


--
-- Name: TABLE v2_job_completed; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.v2_job_completed TO windmill_user;
GRANT ALL ON TABLE public.v2_job_completed TO windmill_admin;


--
-- Name: TABLE v2_as_completed_job; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.v2_as_completed_job TO windmill_user;
GRANT ALL ON TABLE public.v2_as_completed_job TO windmill_admin;


--
-- Name: TABLE v2_job_queue; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.v2_job_queue TO windmill_user;
GRANT ALL ON TABLE public.v2_job_queue TO windmill_admin;


--
-- Name: TABLE v2_job_runtime; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.v2_job_runtime TO windmill_user;
GRANT ALL ON TABLE public.v2_job_runtime TO windmill_admin;


--
-- Name: TABLE v2_job_status; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.v2_job_status TO windmill_user;
GRANT ALL ON TABLE public.v2_job_status TO windmill_admin;


--
-- Name: TABLE v2_as_queue; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.v2_as_queue TO windmill_user;
GRANT ALL ON TABLE public.v2_as_queue TO windmill_admin;


--
-- Name: TABLE variable; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.variable TO windmill_user;
GRANT ALL ON TABLE public.variable TO windmill_admin;


--
-- Name: TABLE websocket_trigger; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.websocket_trigger TO windmill_user;
GRANT ALL ON TABLE public.websocket_trigger TO windmill_admin;


--
-- Name: TABLE windmill_migrations; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.windmill_migrations TO windmill_user;
GRANT ALL ON TABLE public.windmill_migrations TO windmill_admin;


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
-- Name: TABLE workspace_env; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.workspace_env TO windmill_user;
GRANT ALL ON TABLE public.workspace_env TO windmill_admin;


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
-- Name: TABLE zombie_job_counter; Type: ACL; Schema: public; Owner: postgres
--

GRANT ALL ON TABLE public.zombie_job_counter TO windmill_user;
GRANT ALL ON TABLE public.zombie_job_counter TO windmill_admin;


--
-- Name: DEFAULT PRIVILEGES FOR SEQUENCES; Type: DEFAULT ACL; Schema: public; Owner: postgres
--

ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL ON SEQUENCES TO windmill_user;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL ON SEQUENCES TO windmill_admin;


--
-- Name: DEFAULT PRIVILEGES FOR TABLES; Type: DEFAULT ACL; Schema: public; Owner: postgres
--

ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL ON TABLES TO windmill_user;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public GRANT ALL ON TABLES TO windmill_admin;


--
-- PostgreSQL database dump complete
--

