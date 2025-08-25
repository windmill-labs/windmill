-- https://github.com/windmill-labs/windmill/pull/491
CREATE FUNCTION migrate_flow(flow jsonb)
       RETURNS jsonb
AS $$
DECLARE module jsonb;
        i integer := 0;
BEGIN
    if flow->'value'?'modules' THEN
        flow = JSONB_SET(flow, ARRAY['modules'], flow->'value'->'modules') - 'value';
    END IF;

    FOR module IN SELECT JSONB_ARRAY_ELEMENTS(flow->'modules') LOOP
        flow = JSONB_SET(flow, ARRAY['modules', i::text], migrate_flow_module(module));
        i = i + 1;
    END LOOP;

    RETURN flow;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION migrate_flow_module(module jsonb)
       RETURNS jsonb
AS $$
BEGIN
    IF module?'input_transform' AND module->'input_transform' != 'null'::jsonb THEN
        module = JSONB_SET(module, ARRAY['input_transforms'], module->'input_transform')
               - 'input_transform';
    END IF;

    IF module?'stop_after_if_expr' AND module->'stop_after_if_expr' != 'null'::jsonb THEN
        IF NOT module?'stop_after_if' THEN
            module = JSONB_SET(module, ARRAY['stop_after_if'], '{}'::jsonb);
        END IF;
        module = JSONB_SET(module, ARRAY['stop_after_if', 'expr'], module->'stop_after_if_expr')
               - 'stop_after_if_expr';
    END IF;

    IF module?'skip_if_stopped' AND module->'skip_if_stopped' != 'null'::jsonb THEN
        IF NOT module?'stop_after_if' THEN
            module = JSONB_SET(module, ARRAY['stop_after_if'], '{}'::jsonb);
        END IF;
        module = JSONB_SET(module, ARRAY['stop_after_if', 'skip_if_stopped'], module->'skip_if_stopped')
               - 'skip_if_stopped';
    END IF;

    if module->'value'->>'type' = 'forloopflow' THEN
        module = JSONB_SET(module, ARRAY['value'], migrate_flow(module->'value'));
    END IF;

    RETURN module;
END;
$$ LANGUAGE plpgsql;

UPDATE flow SET value = migrate_flow(value);

DROP FUNCTION migrate_flow_module, migrate_flow;
