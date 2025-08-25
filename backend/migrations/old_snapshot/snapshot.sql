--
-- Name: action_kind; 
--

CREATE TYPE action_kind AS ENUM (
    'create',
    'update',
    'delete',
    'execute'
);


--
-- Name: asset_access_type; 
--

CREATE TYPE asset_access_type AS ENUM (
    'r',
    'w',
    'rw'
);


--
-- Name: asset_kind; 
--

CREATE TYPE asset_kind AS ENUM (
    's3object',
    'resource',
    'variable',
    'ducklake'
);


--
-- Name: asset_usage_kind; 
--

CREATE TYPE asset_usage_kind AS ENUM (
    'script',
    'flow'
);


--
-- Name: authentication_method; 
--

CREATE TYPE authentication_method AS ENUM (
    'none',
    'windmill',
    'api_key',
    'basic_http',
    'custom_script',
    'signature'
);


--
-- Name: autoscaling_event_type; 
--

CREATE TYPE autoscaling_event_type AS ENUM (
    'full_scaleout',
    'scalein',
    'scaleout'
);


--
-- Name: aws_auth_resource_type; 
--

CREATE TYPE aws_auth_resource_type AS ENUM (
    'oidc',
    'credentials'
);


--
-- Name: delivery_mode; 
--

CREATE TYPE delivery_mode AS ENUM (
    'push',
    'pull'
);


--
-- Name: draft_type; 
--

CREATE TYPE draft_type AS ENUM (
    'script',
    'flow',
    'app'
);


--
-- Name: favorite_kind; 
--

CREATE TYPE favorite_kind AS ENUM (
    'app',
    'script',
    'flow',
    'raw_app'
);


--
-- Name: gcp_subscription_mode; 
--

CREATE TYPE gcp_subscription_mode AS ENUM (
    'create_update',
    'existing'
);


--
-- Name: http_method; 
--

CREATE TYPE http_method AS ENUM (
    'get',
    'post',
    'put',
    'delete',
    'patch'
);


--
-- Name: importer_kind; 
--

CREATE TYPE importer_kind AS ENUM (
    'script',
    'flow',
    'app'
);


--
-- Name: job_kind; 
--

CREATE TYPE job_kind AS ENUM (
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
    'appscript',
    'aiagent'
);


--
-- Name: job_status; 
--

CREATE TYPE job_status AS ENUM (
    'success',
    'failure',
    'canceled',
    'skipped'
);


--
-- Name: job_trigger_kind; 
--

CREATE TYPE job_trigger_kind AS ENUM (
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
    'gcp',
    'mqtt'
);


--
-- Name: log_mode; 
--

CREATE TYPE log_mode AS ENUM (
    'standalone',
    'server',
    'worker',
    'agent',
    'indexer',
    'mcp'
);


--
-- Name: login_type; 
--

CREATE TYPE login_type AS ENUM (
    'password',
    'github'
);


--
-- Name: metric_kind; 
--

CREATE TYPE metric_kind AS ENUM (
    'scalar_int',
    'scalar_float',
    'timeseries_int',
    'timeseries_float'
);


--
-- Name: mqtt_client_version; 
--

CREATE TYPE mqtt_client_version AS ENUM (
    'v3',
    'v5'
);


--
-- Name: runnable_type; 
--

CREATE TYPE runnable_type AS ENUM (
    'ScriptHash',
    'ScriptPath',
    'FlowPath'
);


--
-- Name: script_kind; 
--

CREATE TYPE script_kind AS ENUM (
    'script',
    'trigger',
    'failure',
    'command',
    'approval',
    'preprocessor'
);


--
-- Name: script_lang; 
--

CREATE TYPE script_lang AS ENUM (
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
    'duckdb',
    'ruby'
);


--
-- Name: trigger_kind; 
--

CREATE TYPE trigger_kind AS ENUM (
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


--
-- Name: workspace_key_kind; 
--

CREATE TYPE workspace_key_kind AS ENUM (
    'cloud'
);


--
-- Name: notify_config_change(); Type: FUNCTION; 
--

CREATE FUNCTION notify_config_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_config_change', NEW.name::text);
    RETURN NEW;
END;
$$;


--
-- Name: notify_global_setting_change(); Type: FUNCTION; 
--

CREATE FUNCTION notify_global_setting_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_global_setting_change', NEW.name::text);
    RETURN NEW;
END;
$$;


--
-- Name: notify_global_setting_delete(); Type: FUNCTION; 
--

CREATE FUNCTION notify_global_setting_delete() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_global_setting_change', OLD.name::text);
    RETURN OLD;
END;
$$;


--
-- Name: notify_http_trigger_change(); Type: FUNCTION; 
--

CREATE FUNCTION notify_http_trigger_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_http_trigger_change', NEW.workspace_id || ':' || NEW.path);
    RETURN NEW;
END;
$$;


--
-- Name: notify_runnable_version_change(); Type: FUNCTION; 
--

CREATE FUNCTION notify_runnable_version_change() RETURNS trigger
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


--
-- Name: notify_token_invalidation(); Type: FUNCTION; 
--

CREATE FUNCTION notify_token_invalidation() RETURNS trigger
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


--
-- Name: notify_webhook_change(); Type: FUNCTION; 
--

CREATE FUNCTION notify_webhook_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_webhook_change', NEW.workspace_id);
    RETURN NEW;
END;
$$;


--
-- Name: notify_workspace_envs_change(); Type: FUNCTION; 
--

CREATE FUNCTION notify_workspace_envs_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_envs_change', NEW.workspace_id);
    RETURN NEW;
END;
$$;


--
-- Name: notify_workspace_premium_change(); Type: FUNCTION; 
--

CREATE FUNCTION notify_workspace_premium_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM pg_notify('notify_workspace_premium_change', NEW.id);
    RETURN NEW;
END;
$$;


--
-- Name: prevent_route_path_change(); Type: FUNCTION; 
--

CREATE FUNCTION prevent_route_path_change() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF CURRENT_USER = 'windmill_user' AND NEW.route_path <> OLD.route_path THEN
        RAISE EXCEPTION 'Modification of route_path is only allowed by admins';
    END IF;
    RETURN NEW;
END;
$$;


--
-- Name: set_session_context(boolean, text, text, text, text, text); Type: FUNCTION; 
--

CREATE FUNCTION set_session_context(admin boolean, username text, groups text, pgroups text, folders_read text, folders_write text) RETURNS void
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

--
-- Name: account; Type: TABLE; 
--

CREATE TABLE account (
    workspace_id character varying(50) NOT NULL,
    id integer NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    refresh_token character varying(1500) NOT NULL,
    client character varying(50) NOT NULL,
    refresh_error text,
    grant_type character varying(50) DEFAULT 'authorization_code'::character varying NOT NULL,
    cc_client_id character varying(500),
    cc_client_secret character varying(500),
    cc_token_url character varying(500)
);


--
-- Name: account_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE account_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: account_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE account_id_seq OWNED BY account.id;


--
-- Name: agent_token_blacklist; Type: TABLE; 
--

CREATE TABLE agent_token_blacklist (
    token character varying NOT NULL,
    expires_at timestamp without time zone NOT NULL,
    blacklisted_at timestamp without time zone DEFAULT now() NOT NULL,
    blacklisted_by character varying NOT NULL
);


--
-- Name: alerts; Type: TABLE; 
--

CREATE TABLE alerts (
    id integer NOT NULL,
    alert_type character varying(50) NOT NULL,
    message text NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    acknowledged boolean,
    workspace_id text,
    acknowledged_workspace boolean,
    resource text
);


--
-- Name: alerts_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE alerts_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: alerts_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE alerts_id_seq OWNED BY alerts.id;


--
-- Name: app; Type: TABLE; 
--

CREATE TABLE app (
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


--
-- Name: app_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE app_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: app_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE app_id_seq OWNED BY app.id;


--
-- Name: app_script; Type: TABLE; 
--

CREATE TABLE app_script (
    id bigint NOT NULL,
    app bigint NOT NULL,
    hash character(64) NOT NULL,
    lock text,
    code text NOT NULL,
    code_sha256 character(64) NOT NULL
);


--
-- Name: app_script_app_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE app_script_app_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: app_script_app_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE app_script_app_seq OWNED BY app_script.app;


--
-- Name: app_script_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE app_script_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: app_script_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE app_script_id_seq OWNED BY app_script.id;


--
-- Name: app_version; Type: TABLE; 
--

CREATE TABLE app_version (
    id bigint NOT NULL,
    app_id bigint NOT NULL,
    value json NOT NULL,
    created_by character varying(50) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    raw_app boolean DEFAULT false NOT NULL
);


--
-- Name: app_version_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE app_version_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: app_version_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE app_version_id_seq OWNED BY app_version.id;


--
-- Name: app_version_lite; Type: TABLE; 
--

CREATE TABLE app_version_lite (
    id bigint NOT NULL,
    value jsonb
);


--
-- Name: app_version_lite_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE app_version_lite_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: app_version_lite_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE app_version_lite_id_seq OWNED BY app_version_lite.id;


--
-- Name: asset; Type: TABLE; 
--

CREATE TABLE asset (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    kind asset_kind NOT NULL,
    usage_access_type asset_access_type,
    usage_path character varying(255) NOT NULL,
    usage_kind asset_usage_kind NOT NULL
);


--
-- Name: audit; Type: TABLE; 
--

CREATE TABLE audit (
    workspace_id character varying(50) NOT NULL,
    id bigint NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL,
    username character varying(255) NOT NULL,
    operation character varying(50) NOT NULL,
    action_kind action_kind NOT NULL,
    resource character varying(255),
    parameters jsonb,
    email character varying(255),
    span character varying(255)
);


--
-- Name: audit_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE audit_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: audit_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE audit_id_seq OWNED BY audit.id;


--
-- Name: autoscaling_event; Type: TABLE; 
--

CREATE TABLE autoscaling_event (
    id integer NOT NULL,
    worker_group text NOT NULL,
    event_type autoscaling_event_type NOT NULL,
    desired_workers integer NOT NULL,
    applied_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    reason text
);


--
-- Name: autoscaling_event_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE autoscaling_event_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: autoscaling_event_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE autoscaling_event_id_seq OWNED BY autoscaling_event.id;


--
-- Name: capture; Type: TABLE; 
--

CREATE TABLE capture (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by character varying(50) NOT NULL,
    main_args jsonb DEFAULT 'null'::jsonb NOT NULL,
    is_flow boolean NOT NULL,
    trigger_kind trigger_kind NOT NULL,
    preprocessor_args jsonb,
    id bigint NOT NULL,
    CONSTRAINT capture_payload_too_big CHECK ((length((main_args)::text) < (512 * 1024)))
);


--
-- Name: capture_config; Type: TABLE; 
--

CREATE TABLE capture_config (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    is_flow boolean NOT NULL,
    trigger_kind trigger_kind NOT NULL,
    trigger_config jsonb,
    owner character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    server_id character varying(50),
    last_client_ping timestamp with time zone,
    last_server_ping timestamp with time zone,
    error text
);


--
-- Name: capture_id_seq; Type: SEQUENCE; 
--

ALTER TABLE capture ALTER COLUMN id ADD GENERATED BY DEFAULT AS IDENTITY (
    SEQUENCE NAME capture_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: cloud_workspace_settings; Type: TABLE; 
--

CREATE TABLE cloud_workspace_settings (
    workspace_id character varying(50) NOT NULL,
    threshold_alert_amount integer,
    last_alert_sent timestamp without time zone,
    last_warning_sent timestamp without time zone
);


--
-- Name: concurrency_counter; Type: TABLE; 
--

CREATE TABLE concurrency_counter (
    concurrency_id character varying(1000) NOT NULL,
    job_uuids jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: concurrency_key; Type: TABLE; 
--

CREATE TABLE concurrency_key (
    key character varying(255) NOT NULL,
    ended_at timestamp with time zone,
    job_id uuid NOT NULL
);


--
-- Name: concurrency_locks; Type: TABLE; 
--

CREATE TABLE concurrency_locks (
    id character varying NOT NULL,
    last_locked_at timestamp without time zone NOT NULL,
    owner character varying
);


--
-- Name: config; Type: TABLE; 
--

CREATE TABLE config (
    name character varying(255) NOT NULL,
    config jsonb DEFAULT '{}'::jsonb
);


--
-- Name: custom_concurrency_key_ended; Type: TABLE; 
--

CREATE TABLE custom_concurrency_key_ended (
    key character varying(255) NOT NULL,
    ended_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: dependency_map; Type: TABLE; 
--

CREATE TABLE dependency_map (
    workspace_id character varying(50) NOT NULL,
    importer_path character varying(510) NOT NULL,
    importer_kind importer_kind NOT NULL,
    imported_path character varying(510) NOT NULL,
    importer_node_id character varying(255) DEFAULT ''::character varying NOT NULL
);


--
-- Name: deployment_metadata; Type: TABLE; 
--

CREATE TABLE deployment_metadata (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    script_hash bigint,
    app_version bigint,
    callback_job_ids uuid[],
    deployment_msg text,
    flow_version bigint
);


--
-- Name: draft; Type: TABLE; 
--

CREATE TABLE draft (
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    typ draft_type NOT NULL,
    value json NOT NULL,
    created_at timestamp without time zone DEFAULT now() NOT NULL
);


--
-- Name: email_to_igroup; Type: TABLE; 
--

CREATE TABLE email_to_igroup (
    email character varying(255) NOT NULL,
    igroup character varying(255) NOT NULL
);


--
-- Name: favorite; Type: TABLE; 
--

CREATE TABLE favorite (
    usr character varying(50) NOT NULL,
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    favorite_kind favorite_kind NOT NULL
);


--
-- Name: flow; Type: TABLE; 
--

CREATE TABLE flow (
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


--
-- Name: flow_node_hash_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE flow_node_hash_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: flow_node; Type: TABLE; 
--

CREATE TABLE flow_node (
    id bigint NOT NULL,
    workspace_id character varying(50) NOT NULL,
    hash bigint,
    path character varying(255) NOT NULL,
    lock text,
    code text,
    flow jsonb,
    hash_v2 character(64) DEFAULT to_hex(nextval('flow_node_hash_seq'::regclass)) NOT NULL
);


--
-- Name: flow_node_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE flow_node_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: flow_node_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE flow_node_id_seq OWNED BY flow_node.id;


--
-- Name: flow_version; Type: TABLE; 
--

CREATE TABLE flow_version (
    id bigint NOT NULL,
    workspace_id character varying(50) NOT NULL,
    path character varying(255) NOT NULL,
    value jsonb NOT NULL,
    schema json,
    created_by character varying(50) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: flow_version_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE flow_version_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: flow_version_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE flow_version_id_seq OWNED BY flow_version.id;


--
-- Name: flow_version_lite; Type: TABLE; 
--

CREATE TABLE flow_version_lite (
    id bigint NOT NULL,
    value jsonb
);


--
-- Name: flow_version_lite_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE flow_version_lite_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: flow_version_lite_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE flow_version_lite_id_seq OWNED BY flow_version_lite.id;


--
-- Name: workspace_runnable_dependencies; Type: TABLE; 
--

CREATE TABLE workspace_runnable_dependencies (
    flow_path character varying(255),
    runnable_path character varying(255) NOT NULL,
    script_hash bigint,
    runnable_is_flow boolean NOT NULL,
    workspace_id character varying(50) NOT NULL,
    app_path character varying(255),
    CONSTRAINT workspace_runnable_dependencies_path_exclusive CHECK ((((flow_path IS NOT NULL) AND (app_path IS NULL)) OR ((flow_path IS NULL) AND (app_path IS NOT NULL))))
);


--
-- Name: flow_workspace_runnables; Type: VIEW; 
--

CREATE VIEW flow_workspace_runnables AS
 SELECT flow_path,
    runnable_path,
    script_hash,
    runnable_is_flow,
    workspace_id
   FROM workspace_runnable_dependencies;


--
-- Name: folder; Type: TABLE; 
--

CREATE TABLE folder (
    name character varying(255) NOT NULL,
    workspace_id character varying(50) NOT NULL,
    display_name character varying(100) NOT NULL,
    owners character varying(255)[] NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    summary text,
    edited_at timestamp with time zone,
    created_by character varying(50)
);


--
-- Name: gcp_trigger; Type: TABLE; 
--

CREATE TABLE gcp_trigger (
    gcp_resource_path character varying(255) NOT NULL,
    topic_id character varying(255) NOT NULL,
    subscription_id character varying(255) NOT NULL,
    delivery_type delivery_mode NOT NULL,
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
    subscription_mode gcp_subscription_mode DEFAULT 'create_update'::gcp_subscription_mode NOT NULL,
    error_handler_path character varying(255),
    error_handler_args jsonb,
    retry jsonb,
    auto_acknowledge_msg boolean DEFAULT true,
    CONSTRAINT gcp_trigger_check CHECK (((delivery_type <> 'push'::delivery_mode) OR (delivery_config IS NOT NULL))),
    CONSTRAINT gcp_trigger_subscription_id_check CHECK (((char_length((subscription_id)::text) >= 3) AND (char_length((subscription_id)::text) <= 255))),
    CONSTRAINT gcp_trigger_topic_id_check CHECK (((char_length((topic_id)::text) >= 3) AND (char_length((topic_id)::text) <= 255)))
);


--
-- Name: global_settings; Type: TABLE; 
--

CREATE TABLE global_settings (
    name character varying(255) NOT NULL,
    value jsonb NOT NULL,
    updated_at timestamp without time zone DEFAULT now() NOT NULL
);


--
-- Name: group_; Type: TABLE; 
--

CREATE TABLE group_ (
    workspace_id character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    summary text,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    CONSTRAINT proper_name CHECK (((name)::text ~ '^[\w-]+$'::text))
);


--
-- Name: healthchecks; Type: TABLE; 
--

CREATE TABLE healthchecks (
    id bigint NOT NULL,
    check_type character varying(50) NOT NULL,
    healthy boolean NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: healthchecks_id_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE healthchecks_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: healthchecks_id_seq; Type: SEQUENCE OWNED BY; 
--

ALTER SEQUENCE healthchecks_id_seq OWNED BY healthchecks.id;


--
-- Name: http_trigger; Type: TABLE; 
--

CREATE TABLE http_trigger (
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
    authentication_method authentication_method DEFAULT 'none'::authentication_method NOT NULL,
    http_method http_method NOT NULL,
    static_asset_config jsonb,
    is_static_website boolean DEFAULT false NOT NULL,
    workspaced_route boolean DEFAULT false NOT NULL,
    wrap_body boolean DEFAULT false NOT NULL,
    raw_string boolean DEFAULT false NOT NULL,
    authentication_resource_path character varying(255) DEFAULT NULL::character varying,
    summary character varying(512),
    description text,
    error_handler_path character varying(255),
    error_handler_args jsonb,
    retry jsonb
);


--
-- Name: http_trigger_version_seq; Type: SEQUENCE; 
--

CREATE SEQUENCE http_trigger_version_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: input; Type: TABLE; 
--

CREATE TABLE input (
    id uuid NOT NULL,
    workspace_id character varying(50) NOT NULL,
    runnable_id character varying(255) NOT NULL,
    runnable_type runnable_type NOT NULL,
    name text NOT NULL,
    args jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    created_by character varying(50) NOT NULL,
    is_public boolean DEFAULT false NOT NULL
);


--
-- Name: instance_group; Type: TABLE; 
--

CREATE TABLE instance_group (
    name character varying(255) NOT NULL,
    summary character varying(2000),
    id character varying(1000),
    scim_display_name character varying(255),
    external_id character varying(512)
);


--
-- Name: job_logs; Type: TABLE; 
--

CREATE TABLE job_logs (
    job_id uuid NOT NULL,
    workspace_id character varying(255),
    created_at timestamp with time zone DEFAULT now(),
    logs text,
    log_offset integer DEFAULT 0 NOT NULL,
    log_file_index text[]
);


--
-- Name: job_perms; Type: TABLE; 
--

CREATE TABLE job_perms (
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


--
-- Name: job_result_stream; Type: TABLE; 
--

CREATE TABLE job_result_stream (
    job_id uuid NOT NULL,
    workspace_id text NOT NULL,
    stream text NOT NULL
);


--
-- Name: job_stats; Type: TABLE; 
--

CREATE TABLE job_stats (
    workspace_id character varying(50) NOT NULL,
    job_id uuid NOT NULL,
    metric_id character varying(50) NOT NULL,
    metric_name character varying(255),
    metric_kind metric_kind NOT NULL,
    scalar_int integer,
    scalar_float real,
    timestamps timestamp with time zone[],
    timeseries_int integer[],
    timeseries_float real[]
);


--
-- Name: kafka_trigger; Type: TABLE; 
--

CREATE TABLE kafka_trigger (
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
    enabled boolean NOT NULL,
    error_handler_path character varying(255),
    error_handler_args jsonb,
    retry jsonb
);


--
-- Name: log_file; Type: TABLE; 
--

CREATE TABLE log_file (
    hostname character varying(255) NOT NULL,
    log_ts timestamp without time zone NOT NULL,
    ok_lines bigint,
    err_lines bigint,
    mode log_mode NOT NULL,
    worker_group character varying(255),
    file_path character varying(510) NOT NULL,
    json_fmt boolean DEFAULT false
);


--
-- Name: magic_link; Type: TABLE; 
--

CREATE TABLE magic_link (
    email character varying(50) NOT NULL,
    token character varying(100) NOT NULL,
    expiration timestamp with time zone DEFAULT (now() + '1 day'::interval) NOT NULL
);


--
-- Name: metrics; Type: TABLE; 
--

CREATE TABLE metrics (
    id character varying(255) NOT NULL,
    value jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: mqtt_trigger; Type: TABLE; 
--

CREATE TABLE mqtt_trigger (
    mqtt_resource_path character varying(255) NOT NULL,
    subscribe_topics jsonb[] NOT NULL,
    client_version mqtt_client_version DEFAULT 'v5'::mqtt_client_version NOT NULL,
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
    enabled boolean NOT NULL,
    error_handler_path character varying(255),
    error_handler_args jsonb,
    retry jsonb
);


--
-- Name: nats_trigger; Type: TABLE; 
--

CREATE TABLE nats_trigger (
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
    enabled boolean NOT NULL,
    error_handler_path character varying(255),
    error_handler_args jsonb,
    retry jsonb
);


--
-- Name: outstanding_wait_time; Type: TABLE; 
--

CREATE TABLE outstanding_wait_time (
    job_id uuid NOT NULL,
    self_wait_time_ms bigint,
    aggregate_wait_time_ms bigint
);


--
-- Name: parallel_monitor_lock; Type: TABLE; 
--

CREATE TABLE parallel_monitor_lock (
    parent_flow_id uuid NOT NULL,
    job_id uuid NOT NULL,
    last_ping timestamp with time zone
);


--
-- Name: password; Type: TABLE; 
--

CREATE TABLE password (
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


--
-- Name: pending_user; Type: TABLE; 
--

CREATE TABLE pending_user (
    email character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    username character varying(50) NOT NULL
);


--
-- Name: pip_resolution_cache; Type: TABLE; 
--

CREATE TABLE pip_resolution_cache (
    hash character varying(255) NOT NULL,
    expiration timestamp without time zone NOT NULL,
    lockfile text NOT NULL
);


--
-- Name: postgres_trigger; Type: TABLE; 
--

CREATE TABLE postgres_trigger (
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
    enabled boolean NOT NULL,
    error_handler_path character varying(255),
    error_handler_args jsonb,
    retry jsonb
);


--
-- Name: raw_app; Type: TABLE; 
--

CREATE TABLE raw_app (
    path character varying(255) NOT NULL,
    version integer DEFAULT 0 NOT NULL,
    workspace_id character varying(50) NOT NULL,
    summary character varying(1000) DEFAULT ''::character varying NOT NULL,
    edited_at timestamp with time zone DEFAULT now() NOT NULL,
    data text NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: resource; Type: TABLE; 
--

CREATE TABLE resource (
    workspace_id character varying(500) NOT NULL,
    path character varying(255) NOT NULL,
    value jsonb,
    description text,
    resource_type character varying(500) NOT NULL,
    extra_perms jsonb DEFAULT '{}'::jsonb NOT NULL,
    edited_at timestamp with time zone,
    created_by character varying(500),
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


--
-- Name: resource_type; Type: TABLE; 
--

CREATE TABLE resource_type (
    workspace_id character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    schema jsonb,
    description text,
    edited_at timestamp with time zone,
    created_by character varying(50),
    format_extension character varying(20),
    CONSTRAINT proper_name CHECK (((name)::text ~ '^[\w-]+$'::text))
);


--
-- Name: resume_job; Type: TABLE; 
--

CREATE TABLE resume_job (
    id uuid NOT NULL,
    job uuid NOT NULL,
    flow uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    value jsonb DEFAULT 'null'::jsonb NOT NULL,
    approver character varying(1000),
    resume_id integer DEFAULT 0 NOT NULL,
    approved boolean DEFAULT true NOT NULL
);


--
-- Name: schedule; Type: TABLE; 
--

CREATE TABLE schedule (
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


--
-- Name: script; Type: TABLE; 
--

CREATE TABLE script (
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
    language script_lang DEFAULT 'python3'::script_lang NOT NULL,
    kind script_kind DEFAULT 'script'::script_kind NOT NULL,
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
    assets jsonb,
    CONSTRAINT proper_id CHECK (((path)::text ~ '^[ufg](\/[\w-]+){2,}$'::text))
);


--
-- Name: sqs_trigger; Type: TABLE; 
--

CREATE TABLE sqs_trigger (
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
    aws_auth_resource_type aws_auth_resource_type DEFAULT 'credentials'::aws_auth_resource_type NOT NULL,
    error_handler_path character varying(255),
    error_handler_args jsonb,
    retry jsonb
);


--
-- Name: token; Type: TABLE; 
--

CREATE TABLE token (
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


--
-- Name: tutorial_progress; Type: TABLE; 
--

CREATE TABLE tutorial_progress (
    email character varying(255) NOT NULL,
    progress bit(64) DEFAULT '0'::"bit" NOT NULL
);


--
-- Name: usage; Type: TABLE; 
--

CREATE TABLE usage (
    id character varying(50) NOT NULL,
    is_workspace boolean NOT NULL,
    month_ integer NOT NULL,
    usage integer NOT NULL
);


--
-- Name: usr; Type: TABLE; 
--

CREATE TABLE usr (
    workspace_id character varying(50) NOT NULL,
    username character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    is_admin boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    operator boolean DEFAULT false NOT NULL,
    disabled boolean DEFAULT false NOT NULL,
    role character varying(50),
    added_via jsonb,
    CONSTRAINT proper_email CHECK (((email)::text ~* '^(?:[a-z0-9!#$%&''*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&''*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$'::text)),
    CONSTRAINT proper_username CHECK (((username)::text ~ '^[\w-]+$'::text))
);


--
-- Name: usr_to_group; Type: TABLE; 
--

CREATE TABLE usr_to_group (
    workspace_id character varying(50) NOT NULL,
    group_ character varying(50) NOT NULL,
    usr character varying(50) DEFAULT 'ruben'::character varying NOT NULL
);


--
-- Name: v2_job; Type: TABLE; 
--

CREATE TABLE v2_job (
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
    kind job_kind DEFAULT 'script'::job_kind NOT NULL,
    runnable_id bigint,
    runnable_path character varying(255),
    parent_job uuid,
    root_job uuid,
    script_lang script_lang DEFAULT 'python3'::script_lang,
    script_entrypoint_override character varying(255),
    flow_step integer,
    flow_step_id character varying(255),
    flow_innermost_root_job uuid,
    trigger character varying(255),
    trigger_kind job_trigger_kind,
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


--
-- Name: v2_job_completed; Type: TABLE; 
--

CREATE TABLE v2_job_completed (
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
    status job_status NOT NULL,
    completed_at timestamp with time zone DEFAULT now() NOT NULL,
    worker character varying(255),
    workflow_as_code_status jsonb,
    result_columns text[],
    retries uuid[],
    extras jsonb
);


--
-- Name: v2_as_completed_job; Type: VIEW; 
--

CREATE VIEW v2_as_completed_job AS
 SELECT j.id,
    j.workspace_id,
    j.parent_job,
    j.created_by,
    j.created_at,
    c.duration_ms,
    ((c.status = 'success'::job_status) OR (c.status = 'skipped'::job_status)) AS success,
    j.runnable_id AS script_hash,
    j.runnable_path AS script_path,
    j.args,
    c.result,
    false AS deleted,
    j.raw_code,
    (c.status = 'canceled'::job_status) AS canceled,
    c.canceled_by,
    c.canceled_reason,
    j.kind AS job_kind,
        CASE
            WHEN (j.trigger_kind = 'schedule'::job_trigger_kind) THEN j.trigger
            ELSE NULL::character varying
        END AS schedule_path,
    j.permissioned_as,
    COALESCE(c.flow_status, c.workflow_as_code_status) AS flow_status,
    j.raw_flow,
    (j.flow_step_id IS NOT NULL) AS is_flow_step,
    j.script_lang AS language,
    c.started_at,
    (c.status = 'skipped'::job_status) AS is_skipped,
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
   FROM (v2_job_completed c
     JOIN v2_job j USING (id));


--
-- Name: v2_job_queue; Type: TABLE; 
--

CREATE TABLE v2_job_queue (
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


--
-- Name: v2_job_runtime; Type: TABLE; 
--

CREATE TABLE v2_job_runtime (
    id uuid NOT NULL,
    ping timestamp with time zone DEFAULT now(),
    memory_peak integer
);


--
-- Name: v2_job_status; Type: TABLE; 
--

CREATE TABLE v2_job_status (
    id uuid NOT NULL,
    flow_status jsonb,
    flow_leaf_jobs jsonb,
    workflow_as_code_status jsonb
);


--
-- Name: v2_as_queue; Type: VIEW; 
--

CREATE VIEW v2_as_queue AS
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
            WHEN (j.trigger_kind = 'schedule'::job_trigger_kind) THEN j.trigger
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
   FROM (((v2_job_queue q
     JOIN v2_job j USING (id))
     LEFT JOIN v2_job_runtime r USING (id))
     LEFT JOIN v2_job_status s USING (id));


--
-- Name: variable; Type: TABLE; 
--

CREATE TABLE variable (
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


--
-- Name: websocket_trigger; Type: TABLE; 
--

CREATE TABLE websocket_trigger (
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
    can_return_message boolean DEFAULT false NOT NULL,
    error_handler_path character varying(255),
    error_handler_args jsonb,
    retry jsonb
);


--
-- Name: windmill_migrations; Type: TABLE; 
--

CREATE TABLE windmill_migrations (
    name text NOT NULL,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP
);


--
-- Name: worker_ping; Type: TABLE; 
--

CREATE TABLE worker_ping (
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


--
-- Name: workspace; Type: TABLE; 
--

CREATE TABLE workspace (
    id character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    owner character varying(50) NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    premium boolean DEFAULT false NOT NULL,
    CONSTRAINT proper_id CHECK (((id)::text ~ '^\w+(-\w+)*$'::text))
);


--
-- Name: workspace_env; Type: TABLE; 
--

CREATE TABLE workspace_env (
    workspace_id character varying(50) NOT NULL,
    name character varying(255) NOT NULL,
    value character varying(1000) NOT NULL
);


--
-- Name: workspace_invite; Type: TABLE; 
--

CREATE TABLE workspace_invite (
    workspace_id character varying(50) NOT NULL,
    email character varying(255) NOT NULL,
    is_admin boolean DEFAULT false NOT NULL,
    operator boolean DEFAULT false NOT NULL,
    CONSTRAINT proper_email CHECK (((email)::text ~* '^(?:[a-z0-9!#$%&''*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&''*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$'::text))
);


--
-- Name: workspace_key; Type: TABLE; 
--

CREATE TABLE workspace_key (
    workspace_id character varying(50) NOT NULL,
    kind workspace_key_kind NOT NULL,
    key character varying(255) DEFAULT 'changeme'::character varying NOT NULL
);


--
-- Name: workspace_settings; Type: TABLE; 
--

CREATE TABLE workspace_settings (
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
    git_app_installations jsonb DEFAULT '[]'::jsonb NOT NULL,
    ducklake jsonb,
    auto_add_instance_groups text[] DEFAULT '{}'::text[],
    auto_add_instance_groups_roles jsonb DEFAULT '{}'::jsonb
);


--
-- Name: zombie_job_counter; Type: TABLE; 
--

CREATE TABLE zombie_job_counter (
    job_id uuid NOT NULL,
    counter integer DEFAULT 0 NOT NULL
);


--
-- Name: account id; Type: DEFAULT; 
--

ALTER TABLE ONLY account ALTER COLUMN id SET DEFAULT nextval('account_id_seq'::regclass);


--
-- Name: alerts id; Type: DEFAULT; 
--

ALTER TABLE ONLY alerts ALTER COLUMN id SET DEFAULT nextval('alerts_id_seq'::regclass);


--
-- Name: app id; Type: DEFAULT; 
--

ALTER TABLE ONLY app ALTER COLUMN id SET DEFAULT nextval('app_id_seq'::regclass);


--
-- Name: app_script id; Type: DEFAULT; 
--

ALTER TABLE ONLY app_script ALTER COLUMN id SET DEFAULT nextval('app_script_id_seq'::regclass);


--
-- Name: app_script app; Type: DEFAULT; 
--

ALTER TABLE ONLY app_script ALTER COLUMN app SET DEFAULT nextval('app_script_app_seq'::regclass);


--
-- Name: app_version id; Type: DEFAULT; 
--

ALTER TABLE ONLY app_version ALTER COLUMN id SET DEFAULT nextval('app_version_id_seq'::regclass);


--
-- Name: app_version_lite id; Type: DEFAULT; 
--

ALTER TABLE ONLY app_version_lite ALTER COLUMN id SET DEFAULT nextval('app_version_lite_id_seq'::regclass);


--
-- Name: audit id; Type: DEFAULT; 
--

ALTER TABLE ONLY audit ALTER COLUMN id SET DEFAULT nextval('audit_id_seq'::regclass);


--
-- Name: autoscaling_event id; Type: DEFAULT; 
--

ALTER TABLE ONLY autoscaling_event ALTER COLUMN id SET DEFAULT nextval('autoscaling_event_id_seq'::regclass);


--
-- Name: flow_node id; Type: DEFAULT; 
--

ALTER TABLE ONLY flow_node ALTER COLUMN id SET DEFAULT nextval('flow_node_id_seq'::regclass);


--
-- Name: flow_version id; Type: DEFAULT; 
--

ALTER TABLE ONLY flow_version ALTER COLUMN id SET DEFAULT nextval('flow_version_id_seq'::regclass);


--
-- Name: flow_version_lite id; Type: DEFAULT; 
--

ALTER TABLE ONLY flow_version_lite ALTER COLUMN id SET DEFAULT nextval('flow_version_lite_id_seq'::regclass);


--
-- Name: healthchecks id; Type: DEFAULT; 
--

ALTER TABLE ONLY healthchecks ALTER COLUMN id SET DEFAULT nextval('healthchecks_id_seq'::regclass);


--
-- Name: account account_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY account
    ADD CONSTRAINT account_pkey PRIMARY KEY (workspace_id, id);


--
-- Name: agent_token_blacklist agent_token_blacklist_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY agent_token_blacklist
    ADD CONSTRAINT agent_token_blacklist_pkey PRIMARY KEY (token);


--
-- Name: alerts alerts_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY alerts
    ADD CONSTRAINT alerts_pkey PRIMARY KEY (id);


--
-- Name: app app_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY app
    ADD CONSTRAINT app_pkey PRIMARY KEY (id);


--
-- Name: app_script app_script_hash_key; Type: CONSTRAINT; 
--

ALTER TABLE ONLY app_script
    ADD CONSTRAINT app_script_hash_key UNIQUE (hash);


--
-- Name: app_script app_script_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY app_script
    ADD CONSTRAINT app_script_pkey PRIMARY KEY (id);


--
-- Name: app_version_lite app_version_lite_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY app_version_lite
    ADD CONSTRAINT app_version_lite_pkey PRIMARY KEY (id);


--
-- Name: app_version app_version_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY app_version
    ADD CONSTRAINT app_version_pkey PRIMARY KEY (id);


--
-- Name: asset asset_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY asset
    ADD CONSTRAINT asset_pkey PRIMARY KEY (workspace_id, path, kind, usage_path, usage_kind);


--
-- Name: audit audit_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY audit
    ADD CONSTRAINT audit_pkey PRIMARY KEY (workspace_id, id);


--
-- Name: autoscaling_event autoscaling_event_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY autoscaling_event
    ADD CONSTRAINT autoscaling_event_pkey PRIMARY KEY (id);


--
-- Name: capture_config capture_config_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY capture_config
    ADD CONSTRAINT capture_config_pkey PRIMARY KEY (workspace_id, path, is_flow, trigger_kind);


--
-- Name: capture capture_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY capture
    ADD CONSTRAINT capture_pkey PRIMARY KEY (id);


--
-- Name: cloud_workspace_settings cloud_workspace_settings_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY cloud_workspace_settings
    ADD CONSTRAINT cloud_workspace_settings_pkey PRIMARY KEY (workspace_id);


--
-- Name: v2_job_completed completed_job_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY v2_job_completed
    ADD CONSTRAINT completed_job_pkey PRIMARY KEY (id);


--
-- Name: concurrency_counter concurrency_counter_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY concurrency_counter
    ADD CONSTRAINT concurrency_counter_pkey PRIMARY KEY (concurrency_id);


--
-- Name: concurrency_key concurrency_key_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY concurrency_key
    ADD CONSTRAINT concurrency_key_pkey PRIMARY KEY (job_id);


--
-- Name: concurrency_locks concurrency_locks_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY concurrency_locks
    ADD CONSTRAINT concurrency_locks_pkey PRIMARY KEY (id);


--
-- Name: custom_concurrency_key_ended custom_concurrency_key_ended_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY custom_concurrency_key_ended
    ADD CONSTRAINT custom_concurrency_key_ended_pkey PRIMARY KEY (key, ended_at);


--
-- Name: dependency_map dependency_map_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY dependency_map
    ADD CONSTRAINT dependency_map_pkey PRIMARY KEY (workspace_id, importer_node_id, importer_kind, importer_path, imported_path);


--
-- Name: draft draft_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY draft
    ADD CONSTRAINT draft_pkey PRIMARY KEY (workspace_id, path, typ);


--
-- Name: email_to_igroup email_to_igroup_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY email_to_igroup
    ADD CONSTRAINT email_to_igroup_pkey PRIMARY KEY (email, igroup);


--
-- Name: favorite favorite_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY favorite
    ADD CONSTRAINT favorite_pkey PRIMARY KEY (usr, workspace_id, favorite_kind, path);


--
-- Name: flow_node flow_node_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY flow_node
    ADD CONSTRAINT flow_node_pkey PRIMARY KEY (id);


--
-- Name: flow_node flow_node_unique_2; Type: CONSTRAINT; 
--

ALTER TABLE ONLY flow_node
    ADD CONSTRAINT flow_node_unique_2 UNIQUE (path, workspace_id, hash_v2);


--
-- Name: flow flow_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY flow
    ADD CONSTRAINT flow_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: flow_version_lite flow_version_lite_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY flow_version_lite
    ADD CONSTRAINT flow_version_lite_pkey PRIMARY KEY (id);


--
-- Name: flow_version flow_version_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY flow_version
    ADD CONSTRAINT flow_version_pkey PRIMARY KEY (id);


--
-- Name: folder folder_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY folder
    ADD CONSTRAINT folder_pkey PRIMARY KEY (workspace_id, name);


--
-- Name: gcp_trigger gcp_trigger_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY gcp_trigger
    ADD CONSTRAINT gcp_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: global_settings global_settings_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY global_settings
    ADD CONSTRAINT global_settings_pkey PRIMARY KEY (name);


--
-- Name: group_ group__pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY group_
    ADD CONSTRAINT group__pkey PRIMARY KEY (workspace_id, name);


--
-- Name: healthchecks healthchecks_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY healthchecks
    ADD CONSTRAINT healthchecks_pkey PRIMARY KEY (id);


--
-- Name: http_trigger http_trigger_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY http_trigger
    ADD CONSTRAINT http_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: input input_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY input
    ADD CONSTRAINT input_pkey PRIMARY KEY (id);


--
-- Name: instance_group instance_group_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY instance_group
    ADD CONSTRAINT instance_group_pkey PRIMARY KEY (name);


--
-- Name: job_logs job_logs_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY job_logs
    ADD CONSTRAINT job_logs_pkey PRIMARY KEY (job_id);


--
-- Name: job_perms job_perms_pk; Type: CONSTRAINT; 
--

ALTER TABLE ONLY job_perms
    ADD CONSTRAINT job_perms_pk PRIMARY KEY (job_id);


--
-- Name: v2_job job_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY v2_job
    ADD CONSTRAINT job_pkey PRIMARY KEY (id);


--
-- Name: job_result_stream job_result_stream_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY job_result_stream
    ADD CONSTRAINT job_result_stream_pkey PRIMARY KEY (job_id);


--
-- Name: job_stats job_stats_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY job_stats
    ADD CONSTRAINT job_stats_pkey PRIMARY KEY (workspace_id, job_id, metric_id);


--
-- Name: kafka_trigger kafka_trigger_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY kafka_trigger
    ADD CONSTRAINT kafka_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: log_file log_file_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY log_file
    ADD CONSTRAINT log_file_pkey PRIMARY KEY (hostname, log_ts);


--
-- Name: magic_link magic_link_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY magic_link
    ADD CONSTRAINT magic_link_pkey PRIMARY KEY (email, token);


--
-- Name: mqtt_trigger mqtt_trigger_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY mqtt_trigger
    ADD CONSTRAINT mqtt_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: nats_trigger nats_trigger_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY nats_trigger
    ADD CONSTRAINT nats_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: outstanding_wait_time outstanding_wait_time_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY outstanding_wait_time
    ADD CONSTRAINT outstanding_wait_time_pkey PRIMARY KEY (job_id);


--
-- Name: parallel_monitor_lock parallel_monitor_lock_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY parallel_monitor_lock
    ADD CONSTRAINT parallel_monitor_lock_pkey PRIMARY KEY (parent_flow_id, job_id);


--
-- Name: password password_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY password
    ADD CONSTRAINT password_pkey PRIMARY KEY (email);


--
-- Name: pending_user pending_user_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY pending_user
    ADD CONSTRAINT pending_user_pkey PRIMARY KEY (email);


--
-- Name: pip_resolution_cache pip_resolution_cache_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY pip_resolution_cache
    ADD CONSTRAINT pip_resolution_cache_pkey PRIMARY KEY (hash);


--
-- Name: postgres_trigger pk_postgres_trigger; Type: CONSTRAINT; 
--

ALTER TABLE ONLY postgres_trigger
    ADD CONSTRAINT pk_postgres_trigger PRIMARY KEY (path, workspace_id);


--
-- Name: sqs_trigger pk_sqs_trigger; Type: CONSTRAINT; 
--

ALTER TABLE ONLY sqs_trigger
    ADD CONSTRAINT pk_sqs_trigger PRIMARY KEY (path, workspace_id);


--
-- Name: v2_job_queue queue_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY v2_job_queue
    ADD CONSTRAINT queue_pkey PRIMARY KEY (id);


--
-- Name: raw_app raw_app_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY raw_app
    ADD CONSTRAINT raw_app_pkey PRIMARY KEY (path);


--
-- Name: resource resource_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY resource
    ADD CONSTRAINT resource_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: resource_type resource_type_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY resource_type
    ADD CONSTRAINT resource_type_pkey PRIMARY KEY (workspace_id, name);


--
-- Name: resume_job resume_job_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY resume_job
    ADD CONSTRAINT resume_job_pkey PRIMARY KEY (id);


--
-- Name: schedule schedule_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY schedule
    ADD CONSTRAINT schedule_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: script script_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY script
    ADD CONSTRAINT script_pkey PRIMARY KEY (workspace_id, hash);


--
-- Name: token token_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY token
    ADD CONSTRAINT token_pkey PRIMARY KEY (token);


--
-- Name: tutorial_progress tutorial_progress_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY tutorial_progress
    ADD CONSTRAINT tutorial_progress_pkey PRIMARY KEY (email);


--
-- Name: app unique_path_workspace_id; Type: CONSTRAINT; 
--

ALTER TABLE ONLY app
    ADD CONSTRAINT unique_path_workspace_id UNIQUE (workspace_id, path);


--
-- Name: usage usage_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY usage
    ADD CONSTRAINT usage_pkey PRIMARY KEY (id, is_workspace, month_);


--
-- Name: usr usr_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY usr
    ADD CONSTRAINT usr_pkey PRIMARY KEY (workspace_id, username);


--
-- Name: usr_to_group usr_to_group_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY usr_to_group
    ADD CONSTRAINT usr_to_group_pkey PRIMARY KEY (workspace_id, usr, group_);


--
-- Name: v2_job_runtime v2_job_runtime_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY v2_job_runtime
    ADD CONSTRAINT v2_job_runtime_pkey PRIMARY KEY (id);


--
-- Name: v2_job_status v2_job_status_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY v2_job_status
    ADD CONSTRAINT v2_job_status_pkey PRIMARY KEY (id);


--
-- Name: variable variable_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY variable
    ADD CONSTRAINT variable_pkey PRIMARY KEY (workspace_id, path);


--
-- Name: websocket_trigger websocket_trigger_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY websocket_trigger
    ADD CONSTRAINT websocket_trigger_pkey PRIMARY KEY (path, workspace_id);


--
-- Name: windmill_migrations windmill_migrations_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY windmill_migrations
    ADD CONSTRAINT windmill_migrations_pkey PRIMARY KEY (name);


--
-- Name: config worker_group_config_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY config
    ADD CONSTRAINT worker_group_config_pkey PRIMARY KEY (name);


--
-- Name: worker_ping worker_ping_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY worker_ping
    ADD CONSTRAINT worker_ping_pkey PRIMARY KEY (worker);


--
-- Name: workspace_env workspace_env_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY workspace_env
    ADD CONSTRAINT workspace_env_pkey PRIMARY KEY (workspace_id, name);


--
-- Name: workspace_invite workspace_invite_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY workspace_invite
    ADD CONSTRAINT workspace_invite_pkey PRIMARY KEY (workspace_id, email);


--
-- Name: workspace_key workspace_key_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY workspace_key
    ADD CONSTRAINT workspace_key_pkey PRIMARY KEY (workspace_id, kind);


--
-- Name: workspace workspace_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY workspace
    ADD CONSTRAINT workspace_pkey PRIMARY KEY (id);


--
-- Name: workspace_settings workspace_settings_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY workspace_settings
    ADD CONSTRAINT workspace_settings_pkey PRIMARY KEY (workspace_id);


--
-- Name: zombie_job_counter zombie_job_counter_pkey; Type: CONSTRAINT; 
--

ALTER TABLE ONLY zombie_job_counter
    ADD CONSTRAINT zombie_job_counter_pkey PRIMARY KEY (job_id);


--
-- Name: alerts_by_workspace; Type: INDEX; 
--

CREATE INDEX alerts_by_workspace ON alerts USING btree (workspace_id);


--
-- Name: app_workspace_with_hash_unique_idx; Type: INDEX; 
--

CREATE UNIQUE INDEX app_workspace_with_hash_unique_idx ON workspace_runnable_dependencies USING btree (app_path, runnable_path, script_hash, runnable_is_flow, workspace_id) WHERE (script_hash IS NOT NULL);


--
-- Name: app_workspace_without_hash_unique_idx; Type: INDEX; 
--

CREATE UNIQUE INDEX app_workspace_without_hash_unique_idx ON workspace_runnable_dependencies USING btree (app_path, runnable_path, runnable_is_flow, workspace_id) WHERE (script_hash IS NULL);


--
-- Name: autoscaling_event_worker_group_idx; Type: INDEX; 
--

CREATE INDEX autoscaling_event_worker_group_idx ON autoscaling_event USING btree (worker_group, applied_at);


--
-- Name: concurrency_key_ended_at_idx; Type: INDEX; 
--

CREATE INDEX concurrency_key_ended_at_idx ON concurrency_key USING btree (key, ended_at DESC);


--
-- Name: dependency_map_imported_path_idx; Type: INDEX; 
--

CREATE INDEX dependency_map_imported_path_idx ON dependency_map USING btree (workspace_id, imported_path);


--
-- Name: deployment_metadata_app; Type: INDEX; 
--

CREATE UNIQUE INDEX deployment_metadata_app ON deployment_metadata USING btree (workspace_id, path, app_version) WHERE (app_version IS NOT NULL);


--
-- Name: deployment_metadata_flow; Type: INDEX; 
--

CREATE UNIQUE INDEX deployment_metadata_flow ON deployment_metadata USING btree (workspace_id, path, flow_version) WHERE (flow_version IS NOT NULL);


--
-- Name: deployment_metadata_script; Type: INDEX; 
--

CREATE UNIQUE INDEX deployment_metadata_script ON deployment_metadata USING btree (workspace_id, script_hash) WHERE (script_hash IS NOT NULL);


--
-- Name: flow_extra_perms; Type: INDEX; 
--

CREATE INDEX flow_extra_perms ON flow USING gin (extra_perms);


--
-- Name: flow_node_hash; Type: INDEX; 
--

CREATE INDEX flow_node_hash ON flow_node USING btree (hash);


--
-- Name: flow_workspace_runnable_path_is_flow_idx; Type: INDEX; 
--

CREATE INDEX flow_workspace_runnable_path_is_flow_idx ON workspace_runnable_dependencies USING btree (runnable_path, runnable_is_flow, workspace_id);


--
-- Name: flow_workspace_with_hash_unique_idx; Type: INDEX; 
--

CREATE UNIQUE INDEX flow_workspace_with_hash_unique_idx ON workspace_runnable_dependencies USING btree (flow_path, runnable_path, script_hash, runnable_is_flow, workspace_id) WHERE (script_hash IS NOT NULL);


--
-- Name: flow_workspace_without_hash_unique_idx; Type: INDEX; 
--

CREATE UNIQUE INDEX flow_workspace_without_hash_unique_idx ON workspace_runnable_dependencies USING btree (flow_path, runnable_path, runnable_is_flow, workspace_id) WHERE (script_hash IS NULL);


--
-- Name: folder_extra_perms; Type: INDEX; 
--

CREATE INDEX folder_extra_perms ON folder USING gin (extra_perms);


--
-- Name: folder_owners; Type: INDEX; 
--

CREATE INDEX folder_owners ON folder USING gin (owners);


--
-- Name: group_extra_perms; Type: INDEX; 
--

CREATE INDEX group_extra_perms ON group_ USING gin (extra_perms);


--
-- Name: healthchecks_check_type_created_at; Type: INDEX; 
--

CREATE INDEX healthchecks_check_type_created_at ON healthchecks USING btree (check_type, created_at);


--
-- Name: idx_agent_token_blacklist_expires_at; Type: INDEX; 
--

CREATE INDEX idx_agent_token_blacklist_expires_at ON agent_token_blacklist USING btree (expires_at);


--
-- Name: idx_asset_kind_path; Type: INDEX; 
--

CREATE INDEX idx_asset_kind_path ON asset USING btree (workspace_id, kind, path);


--
-- Name: idx_asset_usage; Type: INDEX; 
--

CREATE INDEX idx_asset_usage ON asset USING btree (workspace_id, usage_path, usage_kind);


--
-- Name: idx_audit_recent_login_activities; Type: INDEX; 
--

CREATE INDEX idx_audit_recent_login_activities ON audit USING btree ("timestamp", username) WHERE ((operation)::text = ANY ((ARRAY['users.login'::character varying, 'oauth.login'::character varying, 'users.token.refresh'::character varying])::text[]));


--
-- Name: idx_metrics_id_created_at; Type: INDEX; 
--

CREATE INDEX idx_metrics_id_created_at ON metrics USING btree (id, created_at DESC) WHERE ((id)::text ~~ 'queue_%'::text);


--
-- Name: idx_usr_added_via; Type: INDEX; 
--

CREATE INDEX idx_usr_added_via ON usr USING gin (added_via);


--
-- Name: index_flow_version_path_created_at; Type: INDEX; 
--

CREATE INDEX index_flow_version_path_created_at ON flow_version USING btree (path, created_at);


--
-- Name: index_magic_link_exp; Type: INDEX; 
--

CREATE INDEX index_magic_link_exp ON magic_link USING btree (expiration);


--
-- Name: index_script_on_path_created_at; Type: INDEX; 
--

CREATE INDEX index_script_on_path_created_at ON script USING btree (workspace_id, path, created_at DESC);


--
-- Name: index_token_exp; Type: INDEX; 
--

CREATE INDEX index_token_exp ON token USING btree (expiration);


--
-- Name: index_usr_email; Type: INDEX; 
--

CREATE INDEX index_usr_email ON usr USING btree (email);


--
-- Name: ix_audit_timestamps; Type: INDEX; 
--

CREATE INDEX ix_audit_timestamps ON audit USING btree ("timestamp" DESC);


--
-- Name: ix_completed_job_workspace_id_started_at_new_2; Type: INDEX; 
--

CREATE INDEX ix_completed_job_workspace_id_started_at_new_2 ON v2_job_completed USING btree (workspace_id, started_at DESC);


--
-- Name: ix_job_completed_completed_at; Type: INDEX; 
--

CREATE INDEX ix_job_completed_completed_at ON v2_job_completed USING btree (completed_at DESC);


--
-- Name: ix_job_created_at; Type: INDEX; 
--

CREATE INDEX ix_job_created_at ON v2_job USING btree (created_at DESC);


--
-- Name: ix_job_root_job_index_by_path_2; Type: INDEX; 
--

CREATE INDEX ix_job_root_job_index_by_path_2 ON v2_job USING btree (workspace_id, runnable_path, created_at DESC) WHERE (parent_job IS NULL);


--
-- Name: ix_job_workspace_id_created_at_new_3; Type: INDEX; 
--

CREATE INDEX ix_job_workspace_id_created_at_new_3 ON v2_job USING btree (workspace_id, created_at DESC);


--
-- Name: ix_job_workspace_id_created_at_new_5; Type: INDEX; 
--

CREATE INDEX ix_job_workspace_id_created_at_new_5 ON v2_job USING btree (workspace_id, created_at DESC) WHERE ((kind = ANY (ARRAY['preview'::job_kind, 'flowpreview'::job_kind])) AND (parent_job IS NULL));


--
-- Name: ix_job_workspace_id_created_at_new_8; Type: INDEX; 
--

CREATE INDEX ix_job_workspace_id_created_at_new_8 ON v2_job USING btree (workspace_id, created_at DESC) WHERE ((kind = 'deploymentcallback'::job_kind) AND (parent_job IS NULL));


--
-- Name: ix_job_workspace_id_created_at_new_9; Type: INDEX; 
--

CREATE INDEX ix_job_workspace_id_created_at_new_9 ON v2_job USING btree (workspace_id, created_at DESC) WHERE ((kind = ANY (ARRAY['dependencies'::job_kind, 'flowdependencies'::job_kind, 'appdependencies'::job_kind])) AND (parent_job IS NULL));


--
-- Name: ix_v2_job_labels; Type: INDEX; 
--

CREATE INDEX ix_v2_job_labels ON v2_job USING gin (labels) WHERE (labels IS NOT NULL);


--
-- Name: ix_v2_job_workspace_id_created_at; Type: INDEX; 
--

CREATE INDEX ix_v2_job_workspace_id_created_at ON v2_job USING btree (workspace_id, created_at DESC) WHERE ((kind = ANY (ARRAY['script'::job_kind, 'flow'::job_kind, 'singlescriptflow'::job_kind])) AND (parent_job IS NULL));


--
-- Name: job_stats_id; Type: INDEX; 
--

CREATE INDEX job_stats_id ON job_stats USING btree (job_id);


--
-- Name: labeled_jobs_on_jobs; Type: INDEX; 
--

CREATE INDEX labeled_jobs_on_jobs ON v2_job_completed USING gin (((result -> 'wm_labels'::text))) WHERE (result ? 'wm_labels'::text);


--
-- Name: log_file_log_ts_idx; Type: INDEX; 
--

CREATE INDEX log_file_log_ts_idx ON log_file USING btree (log_ts);


--
-- Name: metrics_key_idx; Type: INDEX; 
--

CREATE INDEX metrics_key_idx ON metrics USING btree (id);


--
-- Name: metrics_sort_idx; Type: INDEX; 
--

CREATE INDEX metrics_sort_idx ON metrics USING btree (created_at DESC);


--
-- Name: queue_sort_v2; Type: INDEX; 
--

CREATE INDEX queue_sort_v2 ON v2_job_queue USING btree (priority DESC NULLS LAST, scheduled_for, tag) WHERE (running = false);


--
-- Name: queue_suspended; Type: INDEX; 
--

CREATE INDEX queue_suspended ON v2_job_queue USING btree (priority DESC NULLS LAST, created_at, suspend_until, suspend, tag) WHERE (suspend_until IS NOT NULL);


--
-- Name: resource_extra_perms; Type: INDEX; 
--

CREATE INDEX resource_extra_perms ON resource USING gin (extra_perms);


--
-- Name: root_queue_index_by_path; Type: INDEX; 
--

CREATE INDEX root_queue_index_by_path ON v2_job_queue USING btree (workspace_id, created_at);


--
-- Name: schedule_extra_perms; Type: INDEX; 
--

CREATE INDEX schedule_extra_perms ON schedule USING gin (extra_perms);


--
-- Name: script_extra_perms; Type: INDEX; 
--

CREATE INDEX script_extra_perms ON script USING gin (extra_perms);


--
-- Name: script_not_archived; Type: INDEX; 
--

CREATE INDEX script_not_archived ON script USING btree (workspace_id, path, created_at DESC) WHERE (archived = false);


--
-- Name: unique_subscription_per_gcp_resource; Type: INDEX; 
--

CREATE UNIQUE INDEX unique_subscription_per_gcp_resource ON gcp_trigger USING btree (subscription_id, gcp_resource_path, workspace_id);


--
-- Name: v2_job_queue_suspend; Type: INDEX; 
--

CREATE INDEX v2_job_queue_suspend ON v2_job_queue USING btree (workspace_id, suspend) WHERE (suspend > 0);


--
-- Name: variable_extra_perms; Type: INDEX; 
--

CREATE INDEX variable_extra_perms ON variable USING gin (extra_perms);


--
-- Name: worker_ping_on_ping_at; Type: INDEX; 
--

CREATE INDEX worker_ping_on_ping_at ON worker_ping USING btree (ping_at);


--
-- Name: http_trigger check_route_path_change; Type: TRIGGER; 
--

CREATE TRIGGER check_route_path_change BEFORE UPDATE ON http_trigger FOR EACH ROW EXECUTE FUNCTION prevent_route_path_change();


--
-- Name: flow_version flow_update_trigger; Type: TRIGGER; 
--

CREATE TRIGGER flow_update_trigger AFTER INSERT ON flow_version FOR EACH ROW EXECUTE FUNCTION notify_runnable_version_change('flow');


--
-- Name: http_trigger http_trigger_change_trigger; Type: TRIGGER; 
--

CREATE TRIGGER http_trigger_change_trigger AFTER INSERT OR DELETE OR UPDATE ON http_trigger FOR EACH ROW EXECUTE FUNCTION notify_http_trigger_change();


--
-- Name: config notify_config_change; Type: TRIGGER; 
--

CREATE TRIGGER notify_config_change AFTER INSERT OR UPDATE ON config FOR EACH ROW EXECUTE FUNCTION notify_config_change();


--
-- Name: global_settings notify_global_setting_change; Type: TRIGGER; 
--

CREATE TRIGGER notify_global_setting_change AFTER INSERT OR UPDATE ON global_settings FOR EACH ROW EXECUTE FUNCTION notify_global_setting_change();


--
-- Name: global_settings notify_global_setting_delete; Type: TRIGGER; 
--

CREATE TRIGGER notify_global_setting_delete AFTER DELETE ON global_settings FOR EACH ROW EXECUTE FUNCTION notify_global_setting_delete();


--
-- Name: script script_update_trigger; Type: TRIGGER; 
--

CREATE TRIGGER script_update_trigger AFTER UPDATE OF lock ON script FOR EACH ROW EXECUTE FUNCTION notify_runnable_version_change('script');


--
-- Name: token token_invalidation_trigger; Type: TRIGGER; 
--

CREATE TRIGGER token_invalidation_trigger AFTER DELETE ON token FOR EACH ROW EXECUTE FUNCTION notify_token_invalidation();


--
-- Name: workspace_settings webhook_change_trigger; Type: TRIGGER; 
--

CREATE TRIGGER webhook_change_trigger AFTER UPDATE OF webhook ON workspace_settings FOR EACH ROW WHEN ((old.webhook IS DISTINCT FROM new.webhook)) EXECUTE FUNCTION notify_webhook_change();


--
-- Name: workspace_env workspace_envs_change_trigger; Type: TRIGGER; 
--

CREATE TRIGGER workspace_envs_change_trigger AFTER INSERT OR DELETE OR UPDATE OF name, value ON workspace_env FOR EACH ROW EXECUTE FUNCTION notify_workspace_envs_change();


--
-- Name: workspace workspace_premium_change_trigger; Type: TRIGGER; 
--

CREATE TRIGGER workspace_premium_change_trigger AFTER UPDATE OF premium ON workspace FOR EACH ROW EXECUTE FUNCTION notify_workspace_premium_change();


--
-- Name: account account_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY account
    ADD CONSTRAINT account_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: app_script app_script_app_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY app_script
    ADD CONSTRAINT app_script_app_fkey FOREIGN KEY (app) REFERENCES app(id) ON DELETE CASCADE;


--
-- Name: app_version app_version_flow_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY app_version
    ADD CONSTRAINT app_version_flow_id_fkey FOREIGN KEY (app_id) REFERENCES app(id) ON DELETE CASCADE;


--
-- Name: app_version_lite app_version_lite_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY app_version_lite
    ADD CONSTRAINT app_version_lite_id_fkey FOREIGN KEY (id) REFERENCES app_version(id) ON DELETE CASCADE;


--
-- Name: app app_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY app
    ADD CONSTRAINT app_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: asset asset_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY asset
    ADD CONSTRAINT asset_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- Name: capture_config capture_config_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY capture_config
    ADD CONSTRAINT capture_config_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE;


--
-- Name: capture capture_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY capture
    ADD CONSTRAINT capture_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: cloud_workspace_settings cloud_workspace_settings_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY cloud_workspace_settings
    ADD CONSTRAINT cloud_workspace_settings_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE;


--
-- Name: deployment_metadata deployment_metadata_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY deployment_metadata
    ADD CONSTRAINT deployment_metadata_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: draft draft_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY draft
    ADD CONSTRAINT draft_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: usr_to_group fk_group; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY usr_to_group
    ADD CONSTRAINT fk_group FOREIGN KEY (workspace_id, group_) REFERENCES group_(workspace_id, name);


--
-- Name: job_result_stream fk_job_result_stream_job_id; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY job_result_stream
    ADD CONSTRAINT fk_job_result_stream_job_id FOREIGN KEY (job_id) REFERENCES v2_job_queue(id) ON DELETE CASCADE;


--
-- Name: postgres_trigger fk_postgres_trigger_workspace; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY postgres_trigger
    ADD CONSTRAINT fk_postgres_trigger_workspace FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE;


--
-- Name: sqs_trigger fk_sqs_trigger_workspace; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY sqs_trigger
    ADD CONSTRAINT fk_sqs_trigger_workspace FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE;


--
-- Name: workspace_runnable_dependencies fk_workspace_runnable_dependencies_app_path; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY workspace_runnable_dependencies
    ADD CONSTRAINT fk_workspace_runnable_dependencies_app_path FOREIGN KEY (app_path, workspace_id) REFERENCES app(path, workspace_id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- Name: flow_node flow_node_path_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY flow_node
    ADD CONSTRAINT flow_node_path_workspace_id_fkey FOREIGN KEY (path, workspace_id) REFERENCES flow(path, workspace_id) ON DELETE CASCADE;


--
-- Name: flow_node flow_node_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY flow_node
    ADD CONSTRAINT flow_node_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: flow_version_lite flow_version_lite_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY flow_version_lite
    ADD CONSTRAINT flow_version_lite_id_fkey FOREIGN KEY (id) REFERENCES flow_version(id) ON DELETE CASCADE;


--
-- Name: flow_version flow_version_workspace_id_path_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY flow_version
    ADD CONSTRAINT flow_version_workspace_id_path_fkey FOREIGN KEY (workspace_id, path) REFERENCES flow(workspace_id, path) ON DELETE CASCADE;


--
-- Name: flow flow_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY flow
    ADD CONSTRAINT flow_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: workspace_runnable_dependencies flow_workspace_runnables_workspace_id_flow_path_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY workspace_runnable_dependencies
    ADD CONSTRAINT flow_workspace_runnables_workspace_id_flow_path_fkey FOREIGN KEY (flow_path, workspace_id) REFERENCES flow(path, workspace_id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- Name: folder folder_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY folder
    ADD CONSTRAINT folder_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE;


--
-- Name: group_ group__workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY group_
    ADD CONSTRAINT group__workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: input input_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY input
    ADD CONSTRAINT input_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: job_stats job_stats_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY job_stats
    ADD CONSTRAINT job_stats_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: nats_trigger nats_trigger_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY nats_trigger
    ADD CONSTRAINT nats_trigger_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE;


--
-- Name: raw_app raw_app_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY raw_app
    ADD CONSTRAINT raw_app_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: resource_type resource_type_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY resource_type
    ADD CONSTRAINT resource_type_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: resource resource_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY resource
    ADD CONSTRAINT resource_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: resume_job resume_job_flow_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY resume_job
    ADD CONSTRAINT resume_job_flow_fkey FOREIGN KEY (flow) REFERENCES v2_job_queue(id) ON DELETE CASCADE;


--
-- Name: schedule schedule_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY schedule
    ADD CONSTRAINT schedule_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: script script_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY script
    ADD CONSTRAINT script_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: token token_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY token
    ADD CONSTRAINT token_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: usr_to_group usr_to_group_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY usr_to_group
    ADD CONSTRAINT usr_to_group_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: usr usr_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY usr
    ADD CONSTRAINT usr_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: v2_job_runtime v2_job_runtime_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY v2_job_runtime
    ADD CONSTRAINT v2_job_runtime_id_fkey FOREIGN KEY (id) REFERENCES v2_job_queue(id) ON DELETE CASCADE;


--
-- Name: v2_job_status v2_job_status_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY v2_job_status
    ADD CONSTRAINT v2_job_status_id_fkey FOREIGN KEY (id) REFERENCES v2_job_queue(id) ON DELETE CASCADE;


--
-- Name: variable variable_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY variable
    ADD CONSTRAINT variable_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: workspace_invite workspace_invite_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY workspace_invite
    ADD CONSTRAINT workspace_invite_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: workspace_key workspace_key_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY workspace_key
    ADD CONSTRAINT workspace_key_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: workspace_settings workspace_settings_workspace_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY workspace_settings
    ADD CONSTRAINT workspace_settings_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES workspace(id);


--
-- Name: zombie_job_counter zombie_job_counter_job_id_fkey; Type: FK CONSTRAINT; 
--

ALTER TABLE ONLY zombie_job_counter
    ADD CONSTRAINT zombie_job_counter_job_id_fkey FOREIGN KEY (job_id) REFERENCES v2_job(id) ON DELETE CASCADE;


--
-- Name: account admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON account TO windmill_admin USING (true);


--
-- Name: app admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON app TO windmill_admin USING (true);


--
-- Name: audit admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON audit TO windmill_admin USING (true);


--
-- Name: capture admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON capture TO windmill_admin USING (true);


--
-- Name: capture_config admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON capture_config TO windmill_admin USING (true);


--
-- Name: flow admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON flow TO windmill_admin USING (true);


--
-- Name: folder admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON folder TO windmill_admin USING (true);


--
-- Name: gcp_trigger admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON gcp_trigger TO windmill_admin USING (true);


--
-- Name: http_trigger admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON http_trigger TO windmill_admin USING (true);


--
-- Name: kafka_trigger admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON kafka_trigger TO windmill_admin USING (true);


--
-- Name: mqtt_trigger admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON mqtt_trigger TO windmill_admin USING (true);


--
-- Name: nats_trigger admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON nats_trigger TO windmill_admin USING (true);


--
-- Name: postgres_trigger admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON postgres_trigger TO windmill_admin USING (true);


--
-- Name: raw_app admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON raw_app TO windmill_admin USING (true);


--
-- Name: resource admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON resource TO windmill_admin USING (true);


--
-- Name: schedule admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON schedule TO windmill_admin USING (true);


--
-- Name: script admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON script TO windmill_admin USING (true);


--
-- Name: sqs_trigger admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON sqs_trigger TO windmill_admin USING (true);


--
-- Name: usr_to_group admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON usr_to_group TO windmill_admin USING (true);


--
-- Name: v2_job admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON v2_job TO windmill_admin USING (true);


--
-- Name: v2_job_completed admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON v2_job_completed TO windmill_admin USING (true);


--
-- Name: v2_job_queue admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON v2_job_queue TO windmill_admin USING (true);


--
-- Name: v2_job_runtime admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON v2_job_runtime TO windmill_admin;


--
-- Name: v2_job_status admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON v2_job_status TO windmill_admin;


--
-- Name: variable admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON variable TO windmill_admin USING (true);


--
-- Name: websocket_trigger admin_policy; Type: POLICY; 
--

CREATE POLICY admin_policy ON websocket_trigger TO windmill_admin USING (true);


--
-- Name: app; Type: ROW SECURITY; 
--

ALTER TABLE app ENABLE ROW LEVEL SECURITY;

--
-- Name: audit; Type: ROW SECURITY; 
--

ALTER TABLE audit ENABLE ROW LEVEL SECURITY;

--
-- Name: capture; Type: ROW SECURITY; 
--

ALTER TABLE capture ENABLE ROW LEVEL SECURITY;

--
-- Name: capture_config; Type: ROW SECURITY; 
--

ALTER TABLE capture_config ENABLE ROW LEVEL SECURITY;

--
-- Name: flow; Type: ROW SECURITY; 
--

ALTER TABLE flow ENABLE ROW LEVEL SECURITY;

--
-- Name: folder; Type: ROW SECURITY; 
--

ALTER TABLE folder ENABLE ROW LEVEL SECURITY;

--
-- Name: gcp_trigger; Type: ROW SECURITY; 
--

ALTER TABLE gcp_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: http_trigger; Type: ROW SECURITY; 
--

ALTER TABLE http_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: kafka_trigger; Type: ROW SECURITY; 
--

ALTER TABLE kafka_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: mqtt_trigger; Type: ROW SECURITY; 
--

ALTER TABLE mqtt_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: nats_trigger; Type: ROW SECURITY; 
--

ALTER TABLE nats_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: postgres_trigger; Type: ROW SECURITY; 
--

ALTER TABLE postgres_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: raw_app; Type: ROW SECURITY; 
--

ALTER TABLE raw_app ENABLE ROW LEVEL SECURITY;

--
-- Name: resource; Type: ROW SECURITY; 
--

ALTER TABLE resource ENABLE ROW LEVEL SECURITY;

--
-- Name: audit schedule; Type: POLICY; 
--

CREATE POLICY schedule ON audit FOR INSERT TO windmill_user WITH CHECK (((username)::text ~~ 'schedule-%'::text));


--
-- Name: schedule; Type: ROW SECURITY; 
--

ALTER TABLE schedule ENABLE ROW LEVEL SECURITY;

--
-- Name: audit schedule_audit; Type: POLICY; 
--

CREATE POLICY schedule_audit ON audit FOR INSERT TO windmill_user WITH CHECK (((parameters ->> 'end_user'::text) ~~ 'schedule-%'::text));


--
-- Name: script; Type: ROW SECURITY; 
--

ALTER TABLE script ENABLE ROW LEVEL SECURITY;

--
-- Name: usr_to_group see_extra_perms_groups; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups ON usr_to_group TO windmill_user USING (true) WITH CHECK ((EXISTS ( SELECT f.key,
    f.value
   FROM group_ g,
    LATERAL jsonb_each_text(g.extra_perms) f(key, value)
  WHERE (((usr_to_group.group_)::text = (g.name)::text) AND ((usr_to_group.workspace_id)::text = (g.workspace_id)::text) AND (split_part(f.key, '/'::text, 1) = 'g'::text) AND (f.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (f.value)::boolean))));


--
-- Name: app see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON app FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: flow see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON flow FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(flow.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: folder see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON folder FOR DELETE TO windmill_user USING ((EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))))));


--
-- Name: gcp_trigger see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON gcp_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(gcp_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: kafka_trigger see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON kafka_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(kafka_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: mqtt_trigger see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON mqtt_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(mqtt_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: nats_trigger see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON nats_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(nats_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: postgres_trigger see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON postgres_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(postgres_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: raw_app see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON raw_app FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(raw_app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: resource see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON resource FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(resource.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: schedule see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON schedule FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(schedule.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: script see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON script FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(script.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: sqs_trigger see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON sqs_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(sqs_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: variable see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON variable FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(variable.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: websocket_trigger see_extra_perms_groups_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_delete ON websocket_trigger FOR DELETE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(websocket_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: app see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON app FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: flow see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON flow FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(flow.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: folder see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON folder FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))))));


--
-- Name: gcp_trigger see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON gcp_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(gcp_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: kafka_trigger see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON kafka_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(kafka_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: mqtt_trigger see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON mqtt_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(mqtt_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: nats_trigger see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON nats_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(nats_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: postgres_trigger see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON postgres_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(postgres_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: raw_app see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON raw_app FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(raw_app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: resource see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON resource FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(resource.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: schedule see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON schedule FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(schedule.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: script see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON script FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(script.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: sqs_trigger see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON sqs_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(sqs_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: variable see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON variable FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(variable.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: websocket_trigger see_extra_perms_groups_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_insert ON websocket_trigger FOR INSERT TO windmill_user WITH CHECK ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(websocket_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: app see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON app FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: flow see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON flow FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: folder see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON folder FOR SELECT TO windmill_user USING (((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)) OR (EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)))))));


--
-- Name: gcp_trigger see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON gcp_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: http_trigger see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON http_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: kafka_trigger see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON kafka_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: mqtt_trigger see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON mqtt_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: nats_trigger see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON nats_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: postgres_trigger see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON postgres_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: raw_app see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON raw_app FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: resource see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON resource FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: schedule see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON schedule FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: script see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON script FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: sqs_trigger see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON sqs_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: variable see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON variable FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: websocket_trigger see_extra_perms_groups_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_select ON websocket_trigger FOR SELECT TO windmill_user USING ((extra_perms ?| regexp_split_to_array(current_setting('session.pgroups'::text), ','::text)));


--
-- Name: app see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON app FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: flow see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON flow FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(flow.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: folder see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON folder FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT o.o
   FROM unnest(folder.owners) o(o)
  WHERE ((o.o)::text = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))))));


--
-- Name: gcp_trigger see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON gcp_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(gcp_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: http_trigger see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON http_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(http_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: kafka_trigger see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON kafka_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(kafka_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: mqtt_trigger see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON mqtt_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(mqtt_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: nats_trigger see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON nats_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(nats_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: postgres_trigger see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON postgres_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(postgres_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: raw_app see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON raw_app FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(raw_app.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: resource see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON resource FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(resource.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: schedule see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON schedule FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(schedule.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: script see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON script FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(script.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: sqs_trigger see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON sqs_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(sqs_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: variable see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON variable FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(variable.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: websocket_trigger see_extra_perms_groups_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_groups_update ON websocket_trigger FOR UPDATE TO windmill_user USING ((EXISTS ( SELECT jsonb_each_text.key,
    jsonb_each_text.value
   FROM jsonb_each_text(websocket_trigger.extra_perms) jsonb_each_text(key, value)
  WHERE ((split_part(jsonb_each_text.key, '/'::text, 1) = 'g'::text) AND (jsonb_each_text.key = ANY (regexp_split_to_array(current_setting('session.pgroups'::text), ','::text))) AND (jsonb_each_text.value)::boolean))));


--
-- Name: usr_to_group see_extra_perms_user; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user ON usr_to_group TO windmill_user USING (true) WITH CHECK ((EXISTS ( SELECT 1
   FROM group_
  WHERE (((usr_to_group.group_)::text = (group_.name)::text) AND ((usr_to_group.workspace_id)::text = (group_.workspace_id)::text) AND ((group_.extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean))));


--
-- Name: app see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON app FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: flow see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON flow FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: folder see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON folder FOR DELETE TO windmill_user USING ((concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[])));


--
-- Name: gcp_trigger see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON gcp_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: kafka_trigger see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON kafka_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: mqtt_trigger see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON mqtt_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: nats_trigger see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON nats_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: postgres_trigger see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON postgres_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: raw_app see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON raw_app FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: resource see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON resource FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: schedule see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON schedule FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: script see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON script FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: sqs_trigger see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON sqs_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: variable see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON variable FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: websocket_trigger see_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_delete ON websocket_trigger FOR DELETE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: app see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON app FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: flow see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON flow FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: folder see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON folder FOR INSERT TO windmill_user WITH CHECK ((concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[])));


--
-- Name: gcp_trigger see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON gcp_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: kafka_trigger see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON kafka_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: mqtt_trigger see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON mqtt_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: nats_trigger see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON nats_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: postgres_trigger see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON postgres_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: raw_app see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON raw_app FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: resource see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON resource FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: schedule see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON schedule FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: script see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON script FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: sqs_trigger see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON sqs_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: variable see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON variable FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: websocket_trigger see_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_insert ON websocket_trigger FOR INSERT TO windmill_user WITH CHECK (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: app see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON app FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: flow see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON flow FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: folder see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON folder FOR SELECT TO windmill_user USING (((extra_perms ? concat('u/', current_setting('session.user'::text))) OR (concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[]))));


--
-- Name: gcp_trigger see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON gcp_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: http_trigger see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON http_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: kafka_trigger see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON kafka_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: mqtt_trigger see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON mqtt_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: nats_trigger see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON nats_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: postgres_trigger see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON postgres_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: raw_app see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON raw_app FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: resource see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON resource FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: schedule see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON schedule FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: script see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON script FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: sqs_trigger see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON sqs_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: variable see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON variable FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: websocket_trigger see_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_select ON websocket_trigger FOR SELECT TO windmill_user USING ((extra_perms ? concat('u/', current_setting('session.user'::text))));


--
-- Name: app see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON app FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: flow see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON flow FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: folder see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON folder FOR UPDATE TO windmill_user USING ((concat('u/', current_setting('session.user'::text)) = ANY ((owners)::text[])));


--
-- Name: gcp_trigger see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON gcp_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: http_trigger see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON http_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: kafka_trigger see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON kafka_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: mqtt_trigger see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON mqtt_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: nats_trigger see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON nats_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: postgres_trigger see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON postgres_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: raw_app see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON raw_app FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: resource see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON resource FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: schedule see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON schedule FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: script see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON script FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: sqs_trigger see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON sqs_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: variable see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON variable FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: websocket_trigger see_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_extra_perms_user_update ON websocket_trigger FOR UPDATE TO windmill_user USING (((extra_perms ->> concat('u/', current_setting('session.user'::text))))::boolean);


--
-- Name: v2_job see_folder_extra_perms_user; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user ON v2_job TO windmill_user USING (((visible_to_owner IS TRUE) AND (split_part((runnable_path)::text, '/'::text, 1) = 'f'::text) AND (split_part((runnable_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: app see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON app FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON capture FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture_config see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON capture_config FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: flow see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON flow FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: gcp_trigger see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON gcp_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: kafka_trigger see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON kafka_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: mqtt_trigger see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON mqtt_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: nats_trigger see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON nats_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: postgres_trigger see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON postgres_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: raw_app see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON raw_app FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: resource see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON resource FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: schedule see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON schedule FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: script see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON script FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: sqs_trigger see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON sqs_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: variable see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON variable FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: websocket_trigger see_folder_extra_perms_user_delete; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_delete ON websocket_trigger FOR DELETE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: app see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON app FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON capture FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture_config see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON capture_config FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: flow see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON flow FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: gcp_trigger see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON gcp_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: kafka_trigger see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON kafka_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: mqtt_trigger see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON mqtt_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: nats_trigger see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON nats_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: postgres_trigger see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON postgres_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: raw_app see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON raw_app FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: resource see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON resource FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: schedule see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON schedule FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: script see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON script FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: sqs_trigger see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON sqs_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: variable see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON variable FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: websocket_trigger see_folder_extra_perms_user_insert; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_insert ON websocket_trigger FOR INSERT TO windmill_user WITH CHECK (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: app see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON app FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: capture see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON capture FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: capture_config see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON capture_config FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: flow see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON flow FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: gcp_trigger see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON gcp_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: http_trigger see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON http_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: kafka_trigger see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON kafka_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: mqtt_trigger see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON mqtt_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: nats_trigger see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON nats_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: postgres_trigger see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON postgres_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: raw_app see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON raw_app FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: resource see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON resource FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: schedule see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON schedule FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: script see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON script FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: sqs_trigger see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON sqs_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: variable see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON variable FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: websocket_trigger see_folder_extra_perms_user_select; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_select ON websocket_trigger FOR SELECT TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_read'::text), ','::text)))));


--
-- Name: app see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON app FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON capture FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture_config see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON capture_config FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: flow see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON flow FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: gcp_trigger see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON gcp_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: http_trigger see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON http_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: kafka_trigger see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON kafka_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: mqtt_trigger see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON mqtt_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: nats_trigger see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON nats_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: postgres_trigger see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON postgres_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: raw_app see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON raw_app FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: resource see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON resource FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: schedule see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON schedule FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: script see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON script FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: sqs_trigger see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON sqs_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: variable see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON variable FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: websocket_trigger see_folder_extra_perms_user_update; Type: POLICY; 
--

CREATE POLICY see_folder_extra_perms_user_update ON websocket_trigger FOR UPDATE TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'f'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.folders_write'::text), ','::text)))));


--
-- Name: capture see_from_allowed_runnables; Type: POLICY; 
--

CREATE POLICY see_from_allowed_runnables ON capture TO windmill_user USING (((is_flow AND (EXISTS ( SELECT 1
   FROM flow
  WHERE (((flow.workspace_id)::text = (capture.workspace_id)::text) AND ((flow.path)::text = (capture.path)::text))))) OR ((NOT is_flow) AND (EXISTS ( SELECT 1
   FROM script
  WHERE (((script.workspace_id)::text = (capture.workspace_id)::text) AND ((script.path)::text = (capture.path)::text)))))));


--
-- Name: capture_config see_from_allowed_runnables; Type: POLICY; 
--

CREATE POLICY see_from_allowed_runnables ON capture_config TO windmill_user USING (((is_flow AND (EXISTS ( SELECT 1
   FROM flow
  WHERE (((flow.workspace_id)::text = (capture_config.workspace_id)::text) AND ((flow.path)::text = (capture_config.path)::text))))) OR ((NOT is_flow) AND (EXISTS ( SELECT 1
   FROM script
  WHERE (((script.workspace_id)::text = (capture_config.workspace_id)::text) AND ((script.path)::text = (capture_config.path)::text)))))));


--
-- Name: app see_member; Type: POLICY; 
--

CREATE POLICY see_member ON app TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: capture see_member; Type: POLICY; 
--

CREATE POLICY see_member ON capture TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: capture_config see_member; Type: POLICY; 
--

CREATE POLICY see_member ON capture_config TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: flow see_member; Type: POLICY; 
--

CREATE POLICY see_member ON flow TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: gcp_trigger see_member; Type: POLICY; 
--

CREATE POLICY see_member ON gcp_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: kafka_trigger see_member; Type: POLICY; 
--

CREATE POLICY see_member ON kafka_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: mqtt_trigger see_member; Type: POLICY; 
--

CREATE POLICY see_member ON mqtt_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: nats_trigger see_member; Type: POLICY; 
--

CREATE POLICY see_member ON nats_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: postgres_trigger see_member; Type: POLICY; 
--

CREATE POLICY see_member ON postgres_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: raw_app see_member; Type: POLICY; 
--

CREATE POLICY see_member ON raw_app TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: resource see_member; Type: POLICY; 
--

CREATE POLICY see_member ON resource TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: schedule see_member; Type: POLICY; 
--

CREATE POLICY see_member ON schedule TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: script see_member; Type: POLICY; 
--

CREATE POLICY see_member ON script TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: sqs_trigger see_member; Type: POLICY; 
--

CREATE POLICY see_member ON sqs_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: v2_job see_member; Type: POLICY; 
--

CREATE POLICY see_member ON v2_job TO windmill_user USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'g'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: variable see_member; Type: POLICY; 
--

CREATE POLICY see_member ON variable TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: websocket_trigger see_member; Type: POLICY; 
--

CREATE POLICY see_member ON websocket_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'g'::text) AND (split_part((path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: v2_job see_member_path; Type: POLICY; 
--

CREATE POLICY see_member_path ON v2_job TO windmill_user USING (((visible_to_owner IS TRUE) AND (split_part((runnable_path)::text, '/'::text, 1) = 'g'::text) AND (split_part((runnable_path)::text, '/'::text, 2) = ANY (regexp_split_to_array(current_setting('session.groups'::text), ','::text)))));


--
-- Name: app see_own; Type: POLICY; 
--

CREATE POLICY see_own ON app TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: audit see_own; Type: POLICY; 
--

CREATE POLICY see_own ON audit TO windmill_user USING (((username)::text = current_setting('session.user'::text)));


--
-- Name: capture see_own; Type: POLICY; 
--

CREATE POLICY see_own ON capture TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: capture_config see_own; Type: POLICY; 
--

CREATE POLICY see_own ON capture_config TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: flow see_own; Type: POLICY; 
--

CREATE POLICY see_own ON flow TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: gcp_trigger see_own; Type: POLICY; 
--

CREATE POLICY see_own ON gcp_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: kafka_trigger see_own; Type: POLICY; 
--

CREATE POLICY see_own ON kafka_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: mqtt_trigger see_own; Type: POLICY; 
--

CREATE POLICY see_own ON mqtt_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: nats_trigger see_own; Type: POLICY; 
--

CREATE POLICY see_own ON nats_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: postgres_trigger see_own; Type: POLICY; 
--

CREATE POLICY see_own ON postgres_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: raw_app see_own; Type: POLICY; 
--

CREATE POLICY see_own ON raw_app TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: resource see_own; Type: POLICY; 
--

CREATE POLICY see_own ON resource TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: schedule see_own; Type: POLICY; 
--

CREATE POLICY see_own ON schedule TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: script see_own; Type: POLICY; 
--

CREATE POLICY see_own ON script TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: sqs_trigger see_own; Type: POLICY; 
--

CREATE POLICY see_own ON sqs_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: v2_job see_own; Type: POLICY; 
--

CREATE POLICY see_own ON v2_job TO windmill_user USING (((split_part((permissioned_as)::text, '/'::text, 1) = 'u'::text) AND (split_part((permissioned_as)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: variable see_own; Type: POLICY; 
--

CREATE POLICY see_own ON variable TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: websocket_trigger see_own; Type: POLICY; 
--

CREATE POLICY see_own ON websocket_trigger TO windmill_user USING (((split_part((path)::text, '/'::text, 1) = 'u'::text) AND (split_part((path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: v2_job see_own_path; Type: POLICY; 
--

CREATE POLICY see_own_path ON v2_job TO windmill_user USING (((visible_to_owner IS TRUE) AND (split_part((runnable_path)::text, '/'::text, 1) = 'u'::text) AND (split_part((runnable_path)::text, '/'::text, 2) = current_setting('session.user'::text))));


--
-- Name: sqs_trigger; Type: ROW SECURITY; 
--

ALTER TABLE sqs_trigger ENABLE ROW LEVEL SECURITY;

--
-- Name: usr_to_group; Type: ROW SECURITY; 
--

ALTER TABLE usr_to_group ENABLE ROW LEVEL SECURITY;

--
-- Name: v2_job; Type: ROW SECURITY; 
--

ALTER TABLE v2_job ENABLE ROW LEVEL SECURITY;

--
-- Name: variable; Type: ROW SECURITY; 
--

ALTER TABLE variable ENABLE ROW LEVEL SECURITY;

--
-- Name: audit webhook; Type: POLICY; 
--

CREATE POLICY webhook ON audit FOR INSERT TO windmill_user WITH CHECK (((username)::text ~~ 'webhook-%'::text));


--
-- Name: websocket_trigger; Type: ROW SECURITY; 
--

ALTER TABLE websocket_trigger ENABLE ROW LEVEL SECURITY;

--
-- PostgreSQL database dump complete
--

