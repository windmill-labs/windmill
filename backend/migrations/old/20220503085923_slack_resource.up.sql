-- Add up migration script here
INSERT INTO resource_type(workspace_id, name, schema, description) VALUES
	('starter', 'slack', '{
    "type": "object",
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "required": [],
    "properties": {
        "token": {
            "type": "string",
            "description": "The slack token"
        }
    }
}', 'A slack token to interact with a specific workspace. Can be obtained from the OAuth integration in the workspace settings.')
	;
