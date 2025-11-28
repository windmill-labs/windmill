-- NOTE: Applied after workspace_dependencies_leafs.sql
-- Python script that uses relative imports
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
from f.leafs.python import main as python_leaf

def main():
    return {"python_import": python_leaf()}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/python_importer', 500005, 'python3', '');

-- TypeScript script that uses relative imports
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
import { main as tsLeaf } from "./leafs/ts";

export async function main() {
    return { ts_import: await tsLeaf() };
}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/ts_importer', 500006, 'nativets', '');

-- Dependency map entries
INSERT INTO dependency_map (workspace_id, importer_path, importer_kind, imported_path, importer_node_id) VALUES ('test-workspace', 'f/python_importer', 'script', 'f/leafs/python', '');
INSERT INTO dependency_map (workspace_id, importer_path, importer_kind, imported_path, importer_node_id) VALUES ('test-workspace', 'f/ts_importer', 'script', 'f/leafs/ts', '');
