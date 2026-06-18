-- Fixture for Bun edge case tests
-- Tests deeply nested imports (level1 -> level2 -> level3)

-- Level 3: Base script (deepest level)
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
export function main() {
  return "level3";
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/nested/level3', 20001, 'bun', '');

-- Level 2: Imports level3 using RELATIVE path (./level3.ts)
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
import { main as level3 } from "./level3.ts";

export function main() {
  return "level2 -> " + level3();
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/nested/level2', 20002, 'bun', '');

-- Level 1: Imports level2 using RELATIVE path (./level2.ts)
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
import { main as level2 } from "./level2.ts";

export function main() {
  return "level1 -> " + level2();
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/nested/level1', 20003, 'bun', '');

-- Script with preprocessor function
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, has_preprocessor) VALUES (
'test-workspace',
'test-user',
'
export function preprocessor(value: number) {
  return { value: value * 2 };
}

export function main(value: number) {
  return value + 100;
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"value":{"type":"number"}},"required":["value"],"type":"object"}',
'Script with preprocessor',
'',
'f/edge_cases/with_preprocessor', 20004, 'bun', '', true);

-- Script with nodejs annotation
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'//nodejs

export function main() {
  return process.version;
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'NodeJS mode script',
'',
'f/edge_cases/nodejs_mode', 20005, 'bun', '');

-- Script with nobundling annotation
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'//nobundling

export function main() {
  return "no bundle";
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'No bundling mode script',
'',
'f/edge_cases/nobundling_mode', 20006, 'bun', '');

-- Script that uses circular-ish import pattern (A imports B, B imports C, test imports A and C)
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
export const SHARED_VALUE = "shared";

export function main() {
  return SHARED_VALUE;
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/circular/shared', 20007, 'bun', '');

-- module_a uses RELATIVE path import (./shared.ts)
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
import { SHARED_VALUE } from "./shared.ts";

export function getValue() {
  return "from_a_" + SHARED_VALUE;
}

export function main() {
  return getValue();
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/circular/module_a', 20008, 'bun', '');

-- module_b uses ABSOLUTE path import (/f/circular/shared.ts)
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
import { SHARED_VALUE } from "/f/circular/shared.ts";

export function getValue() {
  return "from_b_" + SHARED_VALUE;
}

export function main() {
  return getValue();
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/circular/module_b', 20009, 'bun', '');
