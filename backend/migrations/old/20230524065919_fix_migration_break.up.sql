-- Add up migration script here
do
 $$
BEGIN

IF (NOT EXISTS (SELECT workspace_id FROM script  WHERE workspace_id = 'demo' UNION ALL SELECT workspace_id FROM flow  WHERE workspace_id = 'demo' UNION ALL SELECT workspace_id FROM app WHERE workspace_id = 'demo'))
THEN
    DELETE FROM workspace_invite WHERE workspace_id = 'demo';
END IF;
END
$$
