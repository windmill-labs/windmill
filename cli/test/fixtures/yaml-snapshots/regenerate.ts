/**
 * Regenerator for the yaml-snapshot fixtures.
 *
 * Run from the cli directory:
 *
 *   bun run test/fixtures/yaml-snapshots/regenerate.ts
 *
 * Each fixture is built from the JS object below by passing it through
 * `yamlStringify(obj, yamlOptions)` — the exact same path `wmill sync pull`
 * uses to write `flow.yaml` / `app.yaml` / `<script>.script.yaml` to disk.
 *
 * After running this, review the diff carefully:
 *
 * - **No diff** → nothing to do.
 * - **Diff in fixture you intentionally edited** → commit alongside the source
 *   change.
 * - **Diff you didn't expect** → STOP. The yaml library, `yamlOptions`, or a
 *   custom tag handler changed the byte format. Decide whether the change is
 *   intended (then bump `version` in `metadata.ts`, add a migration path,
 *   commit fixtures) or accidental (revert).
 *
 * The test `cli/test/yaml_format_snapshot_unit.test.ts` asserts these
 * fixtures round-trip through `yamlParseContent` + `yamlStringify` without
 * byte changes — so any drift surfaces in CI before reaching users.
 */

import { stringify as yamlStringify } from "yaml";
import { writeFile } from "node:fs/promises";
import * as path from "node:path";
import { yamlOptions } from "../../../src/commands/sync/sync.ts";

interface Fixture {
  filename: string;
  source: Record<string, unknown>;
}

const FIXTURES: Fixture[] = [
  {
    filename: "flow_basic.yaml",
    source: {
      summary: "Basic flow",
      description: "Two sequential script modules",
      value: {
        modules: [
          {
            id: "a",
            value: {
              type: "script",
              input_transforms: {
                x: { type: "static", value: "hello" },
              },
              path: "f/lib/uppercase",
            },
          },
          {
            id: "b",
            value: {
              type: "script",
              input_transforms: {
                input: { type: "javascript", expr: "results.a" },
              },
              path: "f/lib/print",
            },
          },
        ],
      },
      schema: {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        type: "object",
        properties: {},
        required: [],
      },
    },
  },
  {
    filename: "flow_branches_loops.yaml",
    source: {
      summary: "Branches and loops",
      description: "",
      value: {
        modules: [
          {
            id: "loop",
            value: {
              type: "forloopflow",
              iterator: { type: "javascript", expr: "[1,2,3]" },
              modules: [
                {
                  id: "step",
                  value: {
                    type: "rawscript",
                    content: "!inline loop/step.bun.ts",
                    language: "bun",
                    input_transforms: {
                      n: { type: "javascript", expr: "flow_input.iter.value" },
                    },
                  },
                },
              ],
              skip_failures: false,
              parallel: false,
              parallelism: 4,
            },
          },
          {
            id: "branchall",
            value: {
              type: "branchall",
              branches: [
                {
                  modules: [
                    {
                      id: "left",
                      value: {
                        type: "identity",
                      },
                    },
                  ],
                  summary: "Left branch",
                  skip_failure: false,
                  parallel: false,
                },
                {
                  modules: [
                    {
                      id: "right",
                      value: {
                        type: "identity",
                      },
                    },
                  ],
                  summary: "Right branch",
                  skip_failure: false,
                  parallel: false,
                },
              ],
              parallel: true,
            },
          },
          {
            id: "branchone",
            value: {
              type: "branchone",
              branches: [
                {
                  expr: "results.branchall.length > 0",
                  modules: [
                    {
                      id: "case_a",
                      value: { type: "identity" },
                    },
                  ],
                  summary: "Case A",
                },
              ],
              default: [
                {
                  id: "fallback",
                  value: { type: "identity" },
                },
              ],
            },
          },
        ],
      },
      schema: {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        type: "object",
        properties: {},
        required: [],
      },
    },
  },
  {
    filename: "flow_retry_suspend_sleep.yaml",
    source: {
      summary: "Retry / suspend / sleep",
      description: "",
      value: {
        modules: [
          {
            id: "with_retry",
            value: {
              type: "script",
              input_transforms: {},
              path: "f/lib/may_fail",
            },
            retry: {
              constant: { attempts: 3, seconds: 5 },
              exponential: { attempts: 2, multiplier: 2, seconds: 1, random_factor: 0 },
            },
            sleep: { type: "static", value: 1 },
          },
          {
            id: "with_suspend",
            value: {
              type: "script",
              input_transforms: {},
              path: "f/lib/manual_review",
            },
            suspend: {
              required_events: 1,
              timeout: 86400,
              continue_on_disapproved: false,
              hide_cancel: false,
              resume_form: {
                schema: {
                  $schema: "https://json-schema.org/draft/2020-12/schema",
                  type: "object",
                  properties: { approved: { type: "boolean" } },
                  required: ["approved"],
                },
              },
            },
          },
        ],
        failure_module: {
          id: "failure",
          value: {
            type: "script",
            input_transforms: {},
            path: "f/lib/notify_failure",
          },
        },
        preprocessor_module: {
          id: "preprocessor",
          value: {
            type: "rawscript",
            content: "!inline preprocessor.bun.ts",
            language: "bun",
            input_transforms: {},
          },
        },
      },
      schema: {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        type: "object",
        properties: {},
        required: [],
      },
    },
  },
  {
    filename: "flow_notes_groups.yaml",
    source: {
      summary: "Notes and groups (#8641)",
      description: "",
      value: {
        modules: [
          {
            id: "fetch",
            summary: "Fetch product",
            value: {
              type: "script",
              input_transforms: {},
              is_trigger: false,
              path: "f/api/product_get",
            },
          },
          {
            id: "map",
            summary: "Map item",
            value: {
              type: "script",
              input_transforms: {
                bc_item: { type: "javascript", expr: "flow_input.bc_item" },
              },
              is_trigger: false,
              path: "f/mapping/item_to_product",
            },
          },
        ],
        notes: [
          {
            id: "note-abc123",
            type: "group",
            color: "blue",
            contained_node_ids: ["fetch", "map"],
            locked: false,
            text: "These steps must run together",
          },
        ],
        groups: [
          {
            summary: "My group",
            start_id: "fetch",
            end_id: "map",
            color: "green",
          },
        ],
      },
      schema: {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        type: "object",
        properties: {},
        required: [],
      },
    },
  },
  {
    filename: "flow_inline_tags.yaml",
    source: {
      summary: "Inline tag string-prefix form",
      description: "Inline scripts and locks expressed as '!inline path' strings",
      value: {
        modules: [
          {
            id: "step1",
            value: {
              type: "rawscript",
              content: "!inline step1/script.bun.ts",
              lock: "!inline step1/script.lock",
              language: "bun",
              input_transforms: {
                x: { type: "static", value: 1 },
              },
            },
          },
        ],
      },
      schema: {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        type: "object",
        properties: {},
        required: [],
      },
    },
  },
  {
    filename: "app_basic.yaml",
    source: {
      summary: "Dashboard",
      value: {
        grid: [
          {
            data: {
              type: "tablecomponent",
              configuration: {},
              componentInput: {
                type: "runnable",
                fieldType: "any",
                fields: {},
                runnable: {
                  type: "runnableByPath",
                  path: "f/lib/list_users",
                  inlineScriptName: undefined,
                  schema: {
                    $schema: "https://json-schema.org/draft/2020-12/schema",
                    type: "object",
                    properties: {},
                    required: [],
                  },
                },
              },
            },
            id: "table_a",
            "3": { fixed: true, x: 0, y: 0, w: 12, h: 6 },
            "12": { fixed: true, x: 0, y: 0, w: 12, h: 6 },
          },
        ],
        fullscreen: false,
        unusedInlineScripts: [],
        hiddenInlineScripts: [],
        norefreshbar: false,
      },
      policy: {
        triggerables_v2: {},
        execution_mode: "viewer",
        on_behalf_of_email: "admin@windmill.dev",
      },
    },
  },
  {
    filename: "app_raw.yaml",
    source: {
      summary: "Raw HTML app",
      value: {
        files: {
          "index.html": "!inline_fileset index.html",
          "app.js": "!inline_fileset app.js",
        },
        runnables: {
          fetch_data: {
            kind: "runnableByPath",
            path: "f/lib/fetch_data",
            fields: {},
          },
        },
      },
      policy: {
        triggerables_v2: {},
        execution_mode: "viewer",
        on_behalf_of_email: "admin@windmill.dev",
      },
    },
  },
  {
    filename: "script_metadata.script.yaml",
    source: {
      summary: "List active users",
      description: "Returns users active in the last N days.",
      lock: "!inline list_users.lock",
      concurrency_time_window_s: 0,
      kind: "script",
      schema: {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        type: "object",
        order: ["days", "limit"],
        properties: {
          days: {
            type: "integer",
            description: "Lookback window in days",
            default: 30,
          },
          limit: {
            type: "integer",
            description: "Max rows",
            default: 100,
          },
        },
        required: ["days"],
      },
    },
  },
];

async function main() {
  const dir = path.dirname(new URL(import.meta.url).pathname);
  for (const { filename, source } of FIXTURES) {
    const out = yamlStringify(source, yamlOptions);
    const target = path.join(dir, filename);
    await writeFile(target, out, "utf-8");
    console.log(`wrote ${filename} (${out.length} bytes)`);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
