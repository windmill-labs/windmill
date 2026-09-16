-- A deployed flow whose inline bun step is workflow-as-code calling task(), in the
-- shape the deploy leaves behind: the RawScript module rewritten into a flow_node that
-- the step then runs as a FlowScript job. No lock, so the worker resolves
-- windmill-client at run time like the other bun fixtures.
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/wac_flow_script',
'{}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"n":{"type":"integer","description":""}},"required":[],"type":"object"}',
'{"modules":[{"id":"a","value":{"type":"flowscript","id":3000000000000011,"language":"bun","input_transforms":{"n":{"expr":"flow_input.n","type":"javascript"}}}}]}',
'system'
);

INSERT INTO public.flow_node(id, workspace_id, path, hash_v2, lock, code) VALUES (
3000000000000011,
'test-workspace',
'f/system/wac_flow_script',
'0000000000000000000000000000000000000000000000000000000000000011',
NULL,
E'import { workflow, task } from "windmill-client";

const double = task(async (n: number) => {
  return n * 2;
});

export const main = workflow(async (n: number) => {
  const d = await double(n);
  return { doubled: d };
});'
);

-- The same flow's step with two tasks that cache their own result.
INSERT INTO public.flow_node(id, workspace_id, path, hash_v2, lock, code) VALUES (
3000000000000012,
'test-workspace',
'f/system/wac_flow_script',
'0000000000000000000000000000000000000000000000000000000000000012',
NULL,
E'import { workflow, task } from "windmill-client";

const double = task(async (n: number) => {
  return n * 2;
}, { cache_ttl: 60 });
const triple = task(async (n: number) => {
  return n * 3;
}, { cache_ttl: 60 });

export const main = workflow(async (n: number) => {
  const d = await double(n);
  const t = await triple(n);
  return { doubled: d, tripled: t };
});'
);
