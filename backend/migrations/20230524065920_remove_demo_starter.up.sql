-- Add up migration script here
do
 $$
BEGIN

IF (NOT EXISTS (SELECT workspace_id FROM script  WHERE workspace_id = 'demo' UNION ALL SELECT workspace_id FROM flow  WHERE workspace_id = 'demo' UNION ALL SELECT workspace_id FROM app WHERE workspace_id = 'demo'))
THEN
    DELETE FROM usr WHERE workspace_id = 'demo';
    DELETE FROM usr_to_group WHERE workspace_id = 'demo';
    DELETE FROM queue WHERE workspace_id = 'demo';
    DELETE FROM completed_job WHERE workspace_id = 'demo';
    DELETE FROM raw_app WHERE workspace_id = 'demo';
    DELETE FROM variable WHERE workspace_id = 'demo';
    DELETE FROM schedule WHERE workspace_id = 'demo';
    DELETE FROM resource WHERE workspace_id = 'demo';
    DELETE FROM resource_type WHERE workspace_id = 'demo';
    DELETE FROM workspace_key WHERE workspace_id = 'demo';
    DELETE FROM group_ WHERE workspace_id = 'demo';
    DELETE FROM workspace_settings WHERE workspace_id = 'demo';
    DELETE FROM workspace WHERE id = 'demo';
END IF;

IF (NOT EXISTS (SELECT workspace_id FROM script  WHERE workspace_id = 'starter' UNION ALL SELECT workspace_id FROM flow  WHERE workspace_id = 'starter' UNION ALL SELECT workspace_id FROM app WHERE workspace_id = 'starter'))
THEN
    DELETE FROM usr WHERE workspace_id = 'starter';
    DELETE FROM usr_to_group WHERE workspace_id = 'starter';
    DELETE FROM queue WHERE workspace_id = 'starter';
    DELETE FROM completed_job WHERE workspace_id = 'starter';
    DELETE FROM raw_app WHERE workspace_id = 'starter';
    DELETE FROM variable WHERE workspace_id = 'starter';
    DELETE FROM schedule WHERE workspace_id = 'starter';
    DELETE FROM resource WHERE workspace_id = 'starter';
    DELETE FROM resource_type WHERE workspace_id = 'starter';
    DELETE FROM workspace_key WHERE workspace_id = 'starter';
    DELETE FROM group_ WHERE workspace_id = 'starter';
    DELETE FROM workspace_settings WHERE workspace_id = 'starter';
    DELETE FROM workspace WHERE id = 'starter';
END IF;


END
$$

