CREATE OR REPLACE FUNCTION update_string(s text)
RETURNS text
LANGUAGE plpgsql
AS $$
DECLARE
  prefix TEXT := '-- https://www.windmill.dev/docs/getting_started/scripts_quickstart/sql#result-collection
-- result_collection=legacy

';
BEGIN
  RETURN prefix || s;
END;
$$;

CREATE OR REPLACE FUNCTION update_all_modules(obj jsonb)
RETURNS jsonb
LANGUAGE plpgsql
AS $$
DECLARE
  result jsonb;
  k text;
  v jsonb;
BEGIN
  IF jsonb_typeof(obj) = 'object' THEN
    result := '{}'::jsonb;
    
    FOR k, v IN SELECT * FROM jsonb_each(obj)
    LOOP
        IF k = 'content' and jsonb_typeof(v) = 'string' AND obj->>'language' IN ('bigquery', 'postgresql', 'duckdb', 'mssql', 'oracledb', 'snowflake', 'mysql') THEN
            result := result || jsonb_build_object('content', update_string(obj->>'content'));
        ELSE
            result := result || jsonb_build_object(k, update_all_modules(v));
        END IF;
    END LOOP;
    
    RETURN result;
  ELSIF jsonb_typeof(obj) = 'array' AND jsonb_array_length(obj) > 0 THEN
    SELECT jsonb_agg(update_all_modules(elem))
    INTO result
    FROM jsonb_array_elements(obj) elem;
    
    RETURN result;
  ELSE
    RETURN obj;
  END IF;
END;
$$;

-- Run on a flow_version_lite jsonb value. Returns an array of flow_node ids whose languages are SQL.
CREATE OR REPLACE FUNCTION find_sql_flow_nodes_ids(obj jsonb)
RETURNS BIGINT[]
LANGUAGE plpgsql
AS $$
DECLARE
  result BIGINT[] := '{}';
  k text;
  v jsonb;
BEGIN
  IF jsonb_typeof(obj) = 'object' THEN
    IF obj->>'language' IN ('bigquery', 'postgresql', 'duckdb', 'mssql', 'oracledb', 'snowflake', 'mysql') AND jsonb_typeof(obj->'id') = 'number' THEN
        result := result || (obj->>'id')::BIGINT;
    END IF;

    FOR k, v IN SELECT * FROM jsonb_each(obj)
    LOOP
        result := result || find_sql_flow_nodes_ids(v);
    END LOOP;
  ELSIF jsonb_typeof(obj) = 'array' AND jsonb_array_length(obj) > 0 THEN
    SELECT array_agg(result_ids)
    INTO result
    FROM jsonb_array_elements(obj) elem, unnest(find_sql_flow_nodes_ids(elem)) as result_ids;
  END IF;
  RETURN result;
END;
$$;

DO $$
BEGIN
  UPDATE app_version SET value = update_all_modules(value::jsonb)::json;
  UPDATE draft SET value = update_all_modules(value::jsonb)::json;
  UPDATE flow SET value = update_all_modules(value);
  UPDATE flow_version SET value = update_all_modules(value);
  UPDATE flow_node SET code = update_string(code) WHERE id IN (
    SELECT v FROM flow_version_lite, unnest(find_sql_flow_nodes_ids(value)) as v
  );
  UPDATE app_script SET code = update_string(code) WHERE id IN (
    SELECT v FROM app_version_lite, unnest(find_sql_flow_nodes_ids(value)) as v
  );
  UPDATE script SET content = update_string(content) WHERE language IN ('bigquery', 'postgresql', 'duckdb', 'mssql', 'oracledb', 'snowflake', 'mysql');

EXCEPTION WHEN OTHERS THEN
  -- ✅ LOG ERROR WITHOUT STOPPING THE MIGRATION
  RAISE WARNING 'Migration failed: %', SQLERRM;
END;
$$;

DROP FUNCTION IF EXISTS update_all_modules(jsonb);
DROP FUNCTION IF EXISTS update_string(text);
DROP FUNCTION IF EXISTS find_sql_flow_nodes_ids(jsonb);
