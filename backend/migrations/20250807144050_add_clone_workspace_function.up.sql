-- Fix app_script hash collision in clone_workspace_data function
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE OR REPLACE FUNCTION clone_workspace_data(source_workspace_id TEXT, target_workspace_id TEXT, target_username TEXT)
RETURNS VOID AS $$
DECLARE
    source_script RECORD;
    target_script_hash BIGINT;
    source_app RECORD;
    target_app_id BIGINT;
    app_id_mapping JSONB := '{}';
    script_hash_mapping JSONB := '{}';
    source_flow_version RECORD;
    target_flow_version_id BIGINT;
BEGIN
    -- Clone workspace settings (merge with existing basic settings)
    UPDATE workspace_settings 
    SET slack_team_id = source_ws.slack_team_id,
        slack_name = source_ws.slack_name,
        slack_command_script = source_ws.slack_command_script,
        slack_email = source_ws.slack_email,
        auto_invite_domain = source_ws.auto_invite_domain,
        auto_invite_operator = source_ws.auto_invite_operator,
        customer_id = source_ws.customer_id,
        plan = source_ws.plan,
        webhook = source_ws.webhook,
        deploy_to = source_ws.deploy_to,
        error_handler = source_ws.error_handler,
        ai_config = source_ws.ai_config,
        error_handler_extra_args = source_ws.error_handler_extra_args,
        error_handler_muted_on_cancel = source_ws.error_handler_muted_on_cancel,
        large_file_storage = source_ws.large_file_storage,
        git_sync = source_ws.git_sync,
        default_app = source_ws.default_app,
        auto_add = source_ws.auto_add,
        default_scripts = source_ws.default_scripts,
        deploy_ui = source_ws.deploy_ui,
        mute_critical_alerts = source_ws.mute_critical_alerts,
        operator_settings = source_ws.operator_settings,
        teams_command_script = source_ws.teams_command_script,
        teams_team_id = source_ws.teams_team_id,
        teams_team_name = source_ws.teams_team_name,
        git_app_installations = source_ws.git_app_installations
    FROM workspace_settings source_ws 
    WHERE source_ws.workspace_id = source_workspace_id
    AND workspace_settings.workspace_id = target_workspace_id;

    -- Clone workspace environment variables
    INSERT INTO workspace_env (workspace_id, name, value)
    SELECT target_workspace_id, name, value
    FROM workspace_env 
    WHERE workspace_id = source_workspace_id;

    -- Clone custom folders (skip the default ones that are already created)
    INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, summary, edited_at, created_by)
    SELECT target_workspace_id, name, display_name, owners, extra_perms, summary, edited_at, target_username
    FROM folder 
    WHERE workspace_id = source_workspace_id 
    AND name NOT IN ('app_themes', 'app_custom', 'app_groups');

    -- Clone custom groups (skip the default 'all' group)
    INSERT INTO group_ (workspace_id, name, summary, extra_perms)
    SELECT target_workspace_id, name, summary, extra_perms
    FROM group_ 
    WHERE workspace_id = source_workspace_id 
    AND name != 'all';

    -- Clone custom resource types
    INSERT INTO resource_type (workspace_id, name, schema, description, edited_at, created_by, format_extension)
    SELECT target_workspace_id, name, schema, description, edited_at, target_username, format_extension
    FROM resource_type 
    WHERE workspace_id = source_workspace_id;

    -- Clone resources
    INSERT INTO resource (workspace_id, path, value, description, resource_type, extra_perms, edited_at, created_by)
    SELECT target_workspace_id, path, value, description, resource_type, extra_perms, edited_at, target_username
    FROM resource 
    WHERE workspace_id = source_workspace_id
    AND path NOT LIKE 'f/app_themes/%'; -- Skip default theme that's already created

    -- Clone variables
    INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms, account, is_oauth, expires_at)
    SELECT target_workspace_id, path, value, is_secret, description, extra_perms, account, is_oauth, expires_at
    FROM variable 
    WHERE workspace_id = source_workspace_id;

    -- Clone scripts with new hashes to avoid conflicts
    FOR source_script IN 
        SELECT * FROM script WHERE workspace_id = source_workspace_id
    LOOP
        -- Generate new hash for the script
        SELECT COALESCE(MAX(hash), 0) + 1 INTO target_script_hash FROM script;
        
        -- Store the mapping for later reference updates
        script_hash_mapping := jsonb_set(script_hash_mapping, ARRAY[source_script.hash::text], to_jsonb(target_script_hash));
        
        -- Insert script with new hash
        INSERT INTO script (
            workspace_id, hash, path, parent_hashes, summary, description, content,
            created_by, created_at, archived, schema, deleted, is_template,
            extra_perms, lock, lock_error_logs, language, kind, tag, draft_only,
            envs, concurrent_limit, concurrency_time_window_s, cache_ttl,
            dedicated_worker, ws_error_handler_muted, priority, timeout,
            delete_after_use, restart_unless_cancelled, concurrency_key,
            visible_to_runner_only, no_main_func, codebase, has_preprocessor,
            on_behalf_of_email, schema_validation
        ) VALUES (
            target_workspace_id, target_script_hash, source_script.path, source_script.parent_hashes,
            source_script.summary, source_script.description, source_script.content,
            target_username, source_script.created_at, source_script.archived, source_script.schema,
            source_script.deleted, source_script.is_template, source_script.extra_perms,
            source_script.lock, source_script.lock_error_logs, source_script.language,
            source_script.kind, source_script.tag, source_script.draft_only, source_script.envs,
            source_script.concurrent_limit, source_script.concurrency_time_window_s, source_script.cache_ttl,
            source_script.dedicated_worker, source_script.ws_error_handler_muted, source_script.priority,
            source_script.timeout, source_script.delete_after_use, source_script.restart_unless_cancelled,
            source_script.concurrency_key, source_script.visible_to_runner_only, source_script.no_main_func,
            source_script.codebase, source_script.has_preprocessor, source_script.on_behalf_of_email,
            source_script.schema_validation
        );
    END LOOP;

    -- Clone flows
    INSERT INTO flow (
        workspace_id, path, summary, description, value, edited_by, edited_at,
        archived, schema, extra_perms, dependency_job, draft_only, tag,
        ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only,
        concurrency_key, versions, on_behalf_of_email, lock_error_logs
    )
    SELECT target_workspace_id, path, summary, description, value, target_username, edited_at,
           archived, schema, extra_perms, NULL, draft_only, tag,
           ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only,
           concurrency_key, ARRAY[]::bigint[], on_behalf_of_email, lock_error_logs
    FROM flow 
    WHERE workspace_id = source_workspace_id;

    -- Clone flow versions
    FOR source_flow_version IN 
        SELECT * FROM flow_version WHERE workspace_id = source_workspace_id
    LOOP
        INSERT INTO flow_version (workspace_id, path, value, schema, created_by, created_at)
        VALUES (target_workspace_id, source_flow_version.path, source_flow_version.value, 
                source_flow_version.schema, target_username, source_flow_version.created_at)
        RETURNING id INTO target_flow_version_id;
        
        -- Update flow to include this version
        UPDATE flow 
        SET versions = array_append(versions, target_flow_version_id)
        WHERE workspace_id = target_workspace_id AND path = source_flow_version.path;
    END LOOP;

    -- Clone flow nodes (with updated script hashes)
    INSERT INTO flow_node (workspace_id, hash, path, lock, code, flow, hash_v2)
    SELECT target_workspace_id,
           (SELECT COALESCE(MAX(hash), 0) FROM flow_node) + row_number() OVER () AS new_hash,
           source_fn.path, source_fn.lock, source_fn.code, source_fn.flow, source_fn.hash_v2
    FROM flow_node source_fn
    WHERE source_fn.workspace_id = source_workspace_id;

    -- Clone applications with new IDs
    FOR source_app IN 
        SELECT * FROM app WHERE workspace_id = source_workspace_id
    LOOP
        INSERT INTO app (workspace_id, path, summary, policy, versions, extra_perms, draft_only, custom_path)
        VALUES (target_workspace_id, source_app.path, source_app.summary, source_app.policy,
                ARRAY[]::bigint[], source_app.extra_perms, source_app.draft_only, source_app.custom_path)
        RETURNING id INTO target_app_id;
        
        -- Store the mapping for app versions
        app_id_mapping := jsonb_set(app_id_mapping, ARRAY[source_app.id::text], to_jsonb(target_app_id));
    END LOOP;

    -- Clone app versions
    INSERT INTO app_version (app_id, value, created_by, created_at, raw_app)
    SELECT (app_id_mapping->>source_av.app_id::text)::bigint, source_av.value, target_username, source_av.created_at, source_av.raw_app
    FROM app_version source_av
    WHERE source_av.app_id IN (SELECT id FROM app WHERE workspace_id = source_workspace_id);

    -- Update app versions arrays
    UPDATE app SET versions = (
        SELECT array_agg(av.id ORDER BY av.created_at)
        FROM app_version av 
        WHERE av.app_id = app.id
    ) WHERE workspace_id = target_workspace_id;

    -- Clone app scripts with recomputed hashes (hash includes app ID)
    INSERT INTO app_script (app, hash, lock, code, code_sha256)
    SELECT 
        (app_id_mapping->>source_as.app::text)::bigint as new_app_id,
        encode(digest(
            decode(lpad(to_hex((app_id_mapping->>source_as.app::text)::bigint), 16, '0'), 'hex') || 
            decode(source_as.code_sha256, 'hex') || 
            coalesce(source_as.lock::bytea, ''::bytea)
        , 'sha256'), 'hex') as new_hash,
        source_as.lock, 
        source_as.code, 
        source_as.code_sha256
    FROM app_script source_as
    WHERE source_as.app IN (SELECT id FROM app WHERE workspace_id = source_workspace_id);

    -- Clone raw apps
    INSERT INTO raw_app (path, version, workspace_id, summary, edited_at, data, extra_perms)
    SELECT path, version, target_workspace_id, summary, edited_at, data, extra_perms
    FROM raw_app 
    WHERE workspace_id = source_workspace_id;

    -- Clone schedules
    INSERT INTO schedule (
        workspace_id, path, edited_by, edited_at, schedule, enabled, script_path,
        args, extra_perms, is_flow, email, error, timezone, on_failure, on_recovery,
        on_failure_times, on_failure_exact, on_failure_extra_args, on_recovery_times,
        on_recovery_extra_args, ws_error_handler_muted, retry, summary, no_flow_overlap,
        tag, paused_until, on_success, on_success_extra_args, cron_version, description
    )
    SELECT target_workspace_id, path, target_username, edited_at, schedule, enabled, script_path,
           args, extra_perms, is_flow, email, error, timezone, on_failure, on_recovery,
           on_failure_times, on_failure_exact, on_failure_extra_args, on_recovery_times,
           on_recovery_extra_args, ws_error_handler_muted, retry, summary, no_flow_overlap,
           tag, paused_until, on_success, on_success_extra_args, cron_version, description
    FROM schedule 
    WHERE workspace_id = source_workspace_id;

    -- Clone HTTP triggers
    INSERT INTO http_trigger (
        path, route_path, route_path_key, script_path, is_flow, workspace_id,
        edited_by, email, edited_at, extra_perms, is_async, authentication_method,
        http_method, static_asset_config, is_static_website, workspaced_route,
        wrap_body, raw_string, authentication_resource_path
    )
    SELECT path, route_path, route_path_key, script_path, is_flow, target_workspace_id,
           target_username, email, edited_at, extra_perms, is_async, authentication_method,
           http_method, static_asset_config, is_static_website, workspaced_route,
           wrap_body, raw_string, authentication_resource_path
    FROM http_trigger 
    WHERE workspace_id = source_workspace_id;

    -- Clone WebSocket triggers
    INSERT INTO websocket_trigger (
        path, url, script_path, is_flow, workspace_id, edited_by, email,
        edited_at, extra_perms, server_id, last_server_ping, error, enabled,
        filters, initial_messages, url_runnable_args, can_return_message
    )
    SELECT path, url, script_path, is_flow, target_workspace_id, target_username, email,
           edited_at, extra_perms, server_id, last_server_ping, error, enabled,
           filters, initial_messages, url_runnable_args, can_return_message
    FROM websocket_trigger 
    WHERE workspace_id = source_workspace_id;

    -- Clone Kafka triggers
    INSERT INTO kafka_trigger (
        path, kafka_resource_path, topics, group_id, script_path, is_flow,
        workspace_id, edited_by, email, edited_at, extra_perms, server_id,
        last_server_ping, error, enabled
    )
    SELECT path, kafka_resource_path, topics, group_id, script_path, is_flow,
           target_workspace_id, target_username, email, edited_at, extra_perms, server_id,
           last_server_ping, error, enabled
    FROM kafka_trigger 
    WHERE workspace_id = source_workspace_id;

    -- Clone MQTT triggers
    INSERT INTO mqtt_trigger (
        mqtt_resource_path, subscribe_topics, client_version, v5_config, v3_config,
        client_id, path, script_path, is_flow, workspace_id, edited_by, email,
        edited_at, extra_perms, server_id, last_server_ping, error, enabled
    )
    SELECT mqtt_resource_path, subscribe_topics, client_version, v5_config, v3_config,
           client_id, path, script_path, is_flow, target_workspace_id, target_username, email,
           edited_at, extra_perms, server_id, last_server_ping, error, enabled
    FROM mqtt_trigger 
    WHERE workspace_id = source_workspace_id;

    -- Clone NATS triggers
    INSERT INTO nats_trigger (
        path, nats_resource_path, subjects, stream_name, consumer_name,
        use_jetstream, script_path, is_flow, workspace_id, edited_by, email,
        edited_at, extra_perms, server_id, last_server_ping, error, enabled
    )
    SELECT path, nats_resource_path, subjects, stream_name, consumer_name,
           use_jetstream, script_path, is_flow, target_workspace_id, target_username, email,
           edited_at, extra_perms, server_id, last_server_ping, error, enabled
    FROM nats_trigger 
    WHERE workspace_id = source_workspace_id;

    -- Clone PostgreSQL triggers
    INSERT INTO postgres_trigger (
        path, script_path, is_flow, workspace_id, edited_by, email, edited_at,
        extra_perms, postgres_resource_path, error, server_id, last_server_ping,
        replication_slot_name, publication_name, enabled
    )
    SELECT path, script_path, is_flow, target_workspace_id, target_username, email, edited_at,
           extra_perms, postgres_resource_path, error, server_id, last_server_ping,
           replication_slot_name, publication_name, enabled
    FROM postgres_trigger 
    WHERE workspace_id = source_workspace_id;

    -- Clone SQS triggers
    INSERT INTO sqs_trigger (
        path, queue_url, aws_resource_path, message_attributes, script_path,
        is_flow, workspace_id, edited_by, email, edited_at, extra_perms,
        error, server_id, last_server_ping, enabled, aws_auth_resource_type
    )
    SELECT path, queue_url, aws_resource_path, message_attributes, script_path,
           is_flow, target_workspace_id, target_username, email, edited_at, extra_perms,
           error, server_id, last_server_ping, enabled, aws_auth_resource_type
    FROM sqs_trigger 
    WHERE workspace_id = source_workspace_id;

    -- Clone GCP triggers
    INSERT INTO gcp_trigger (
        gcp_resource_path, topic_id, subscription_id, delivery_type, delivery_config,
        path, script_path, is_flow, workspace_id, edited_by, email, edited_at,
        extra_perms, server_id, last_server_ping, error, enabled, subscription_mode
    )
    SELECT gcp_resource_path, topic_id, subscription_id, delivery_type, delivery_config,
           path, script_path, is_flow, target_workspace_id, target_username, email, edited_at,
           extra_perms, server_id, last_server_ping, error, enabled, subscription_mode
    FROM gcp_trigger 
    WHERE workspace_id = source_workspace_id;

    -- Clone workspace runnable dependencies (update with new script hashes and app IDs)
    INSERT INTO workspace_runnable_dependencies (
        flow_path, runnable_path, script_hash, runnable_is_flow, workspace_id, app_path
    )
    SELECT flow_path, runnable_path, 
           CASE 
               WHEN script_hash IS NOT NULL AND script_hash_mapping ? script_hash::text 
               THEN (script_hash_mapping->>script_hash::text)::bigint
               ELSE script_hash
           END,
           runnable_is_flow, target_workspace_id, app_path
    FROM workspace_runnable_dependencies 
    WHERE workspace_id = source_workspace_id;

END;
$$ LANGUAGE plpgsql;