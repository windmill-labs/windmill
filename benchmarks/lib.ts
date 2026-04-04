import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";
import * as windmill from "https://deno.land/x/windmill@v1.174.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.174.0/windmill-api/index.ts";

export const VERSION = "v1.674.2";

export async function login(email: string, password: string): Promise<string> {
  return await windmill.UserService.login({
    requestBody: {
      email: email,
      password: password,
    },
  });
}

async function waitForDeployment(workspace: string, hash: string) {
  const maxTries = 20;
  for (let i = 0; i < maxTries; i++) {
    try {
      const resp = await windmill.ScriptService.getScriptDeploymentStatus({
        workspace,
        hash,
      });

      if (resp.lock !== null) {
        return;
      }
    } catch (err) {}
    await sleep(0.5);
  }
  throw new Error("Script did not deploy in time");
}

async function waitForDedicatedWorker(workspace: string, path: string) {
  const query = windmill.JobService.runWaitResultScriptByPath({
    workspace,
    path,
    requestBody: {},
  });
  let timeout;
  const timeoutPromise = new Promise((_, reject) => {
    timeout = setTimeout(() => {
      reject("Timeout");
    }, 15000);
  });
  await Promise.race([query, timeoutPromise]);
  clearTimeout(timeout);
}

export async function createBenchScript(
  scriptPattern: string,
  workspace: string,
) {
  const path = `f/benchmarks/${scriptPattern}`;
  const exists = await windmill.ScriptService.existsScriptByPath({
    workspace,
    path,
  });

  if (exists) {
    await windmill.ScriptService.deleteScriptByPath({
      workspace,
      path,
    });
  }

  let scriptContent: string;
  let language: string;
  let schemaProperties = {};
  if (scriptPattern === "python") {
    scriptContent =
      'import os\n\ndef main():\n    return os.environ.get("WM_JOB_ID")';
    language = "python3";
  } else if (scriptPattern === "go") {
    scriptContent =
      'package inner\nimport "os"\nfunc main() (string, error) { return os.Getenv("WM_JOB_ID"), nil }';
    language = "go";
  } else if (scriptPattern === "bash") {
    scriptContent = "echo $WM_JOB_ID";
    language = "bash";
  } else if (scriptPattern === "bun") {
    scriptContent = 'export function main(){ return Bun.env["WM_JOB_ID"]; }';
    language = "bun";
  } else if (scriptPattern === "dedicated") {
    scriptContent = "export function main(uuid){ return uuid; }";
    language = "bun";
    schemaProperties = {
      uuid: { default: null, description: "", type: "string" },
    };
  } else if (scriptPattern === "deno") {
    scriptContent =
      'export function main(){ return Deno.env.get("WM_JOB_ID"); }';
    language = "deno";
  } else if (scriptPattern === "nativets") {
    scriptContent =
      '//native\nexport async function main(){ return (await fetch(BASE_URL + "/api/version")).text() }';
    language = "bunnative";
  } else if (scriptPattern === "dedicated_nativets") {
    scriptContent = "//native\nexport function main(){ return 42; }";
    language = "bunnative";
  } else {
    throw new Error(
      "Could not create script for script pattern " + scriptPattern,
    );
  }

  const hash = await windmill.ScriptService.createScript({
    workspace,
    requestBody: {
      path,
      content: scriptContent,
      summary: scriptPattern + " benchmark",
      description: "",
      language: language as api.NewScript.language,
      dedicated_worker:
        scriptPattern === "dedicated" || scriptPattern === "dedicated_nativets",
      schema: {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        properties: schemaProperties,
        required: [],
        type: "object",
      },
    },
  });

  await waitForDeployment(workspace, hash);

  console.log("Created benchmark script at path", path);

  if (scriptPattern === "dedicated" || scriptPattern === "dedicated_nativets") {
    await waitForDedicatedWorker(workspace, path);
  }
}

// WAC v2 benchmark script content patterns
const WAC_SCRIPTS: Record<string, string> = {
  wac_seq_2: [
    'import { task, workflow } from "windmill-client";',
    "const step_a = task(async () => { return 1; });",
    "const step_b = task(async () => { return 2; });",
    "export const main = workflow(async () => {",
    "  const a = await step_a();",
    "  const b = await step_b();",
    "  return { a, b };",
    "});",
  ].join("\n"),

  wac_par_2: [
    'import { task, workflow } from "windmill-client";',
    "const step_a = task(async () => { return 1; });",
    "const step_b = task(async () => { return 2; });",
    "export const main = workflow(async () => {",
    "  const [a, b] = await Promise.all([step_a(), step_b()]);",
    "  return { a, b };",
    "});",
  ].join("\n"),

  wac_seq_3: [
    'import { task, workflow } from "windmill-client";',
    "const step_a = task(async () => { return 1; });",
    "const step_b = task(async () => { return 2; });",
    "const step_c = task(async () => { return 3; });",
    "export const main = workflow(async () => {",
    "  const a = await step_a();",
    "  const b = await step_b();",
    "  const c = await step_c();",
    "  return { a, b, c };",
    "});",
  ].join("\n"),

  wac_inline_2: [
    'import { step, workflow } from "windmill-client";',
    "export const main = workflow(async () => {",
    '  const a = await step("a", () => 1);',
    '  const b = await step("b", () => 2);',
    "  return { a, b };",
    "});",
  ].join("\n"),
};

export const WAC_KINDS = Object.keys(WAC_SCRIPTS);

// Number of child jobs created per workflow instance (used to compute throughput)
// For task(): each task creates a child job. For step(): no child job.
// Total completed jobs per workflow = nSteps + 1 (children + parent)
export const STEPS_PER_WORKFLOW: Record<string, number> = {
  wac_seq_2: 2,
  wac_par_2: 2,
  wac_seq_3: 3,
  wac_inline_2: 0, // inline steps don't create child jobs
  flow_seq_2_bun: 2,
  flow_par_2_bun: 2,
  flow_seq_3_bun: 3,
};

export async function createWacBenchScript(
  wacPattern: string,
  workspace: string,
) {
  const scriptContent = WAC_SCRIPTS[wacPattern];
  if (!scriptContent) {
    throw new Error("Unknown WAC pattern: " + wacPattern);
  }

  const path = `f/benchmarks/${wacPattern}`;
  const exists = await windmill.ScriptService.existsScriptByPath({
    workspace,
    path,
  });

  if (exists) {
    await windmill.ScriptService.deleteScriptByPath({
      workspace,
      path,
    });
  }

  const hash = await windmill.ScriptService.createScript({
    workspace,
    requestBody: {
      path,
      content: scriptContent,
      summary: wacPattern + " WAC v2 benchmark",
      description: "",
      language: "bun" as api.NewScript.language,
      schema: {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        properties: {},
        required: [],
        type: "object",
      },
    },
  });

  await waitForDeployment(workspace, hash);
  console.log("Created WAC v2 benchmark script at path", path);
}

export async function loadJsonConfig<T>(configPath: string): Promise<T> {
  if (configPath.startsWith("http")) {
    const response = await fetch(configPath);
    return await response.json();
  } else {
    return JSON.parse(await Deno.readTextFile(configPath));
  }
}

export const getFlowPayload = (flowPattern: string): api.FlowPreview => {
  if (flowPattern == "branchone") {
    return {
      path: "branchone",
      args: {},
      value: {
        modules: [
          {
            id: "a",
            value: {
              input_transforms: {},
              language: api.RawScript.language.DENO,
              type: "rawscript",
              content:
                'export function main(){ return Deno.env.get("WM_FLOW_JOB_ID"); }',
            },
          },
          {
            id: "b",
            value: {
              type: "branchone",
              branches: [],
              default: [
                {
                  id: "c",
                  value: {
                    input_transforms: {
                      x: {
                        type: "javascript",
                        expr: "results.a",
                      },
                    },
                    language: api.RawScript.language.DENO,
                    type: "rawscript",
                    content: "export function main(x: string){ return x; }",
                  },
                },
              ],
            },
          },
        ],
      },
    };
  } else if (flowPattern == "branchallparrallel") {
    return {
      path: "branchall",
      args: {},
      value: {
        modules: [
          {
            id: "a",
            value: {
              input_transforms: {},
              language: api.RawScript.language.DENO,
              type: "rawscript",
              content:
                'export function main(){ return Deno.env.get("WM_FLOW_JOB_ID"); }',
            },
          },
          {
            id: "b",
            value: {
              type: "branchall",
              parallel: true,
              branches: [
                {
                  modules: [
                    {
                      id: "c",
                      value: {
                        input_transforms: {
                          x: {
                            type: "javascript",
                            expr: "results.a",
                          },
                        },
                        language: api.RawScript.language.DENO,
                        type: "rawscript",
                        content: "export function main(x: string){ return x; }",
                      },
                    },
                  ],
                },
                {
                  modules: [
                    {
                      id: "d",
                      value: {
                        input_transforms: {
                          x: {
                            type: "javascript",
                            expr: "results.a",
                          },
                        },
                        language: api.RawScript.language.DENO,
                        type: "rawscript",
                        content: "export function main(x: string){ return x; }",
                      },
                    },
                  ],
                },
              ],
            },
          },
        ],
      },
    };
  } else if (flowPattern == "bigscriptinflow") {
    return {
      path: "bigscriptinflow",
      args: {},
      value: {
        modules: [
          {
            value: {
              input_transforms: {},
              language: api.RawScript.language.BASH,
              type: "rawscript",
              content:
                "# let's bloat that bash script, 3.. 2.. 1.. BOOM\n".repeat(
                  100,
                ) +
                `if [[ -z $\{WM_FLOW_JOB_ID+x\} ]]; then\necho "not set"\nelif [[ -z "$WM_FLOW_JOB_ID" ]]; then\necho "empty"\nelse\necho "$WM_FLOW_JOB_ID"\nfi`,
            },
          },
        ],
      },
    };
  } else if (flowPattern == "flow_seq_2_bun") {
    return {
      path: "flow_seq_2_bun",
      args: {},
      value: {
        modules: [
          {
            id: "a",
            value: {
              input_transforms: {},
              language: "bun" as api.RawScript.language,
              type: "rawscript",
              content: "export function main() { return 1; }",
            },
          },
          {
            id: "b",
            value: {
              input_transforms: {},
              language: "bun" as api.RawScript.language,
              type: "rawscript",
              content: "export function main() { return 2; }",
            },
          },
        ],
      },
    };
  } else if (flowPattern == "flow_par_2_bun") {
    return {
      path: "flow_par_2_bun",
      args: {},
      value: {
        modules: [
          {
            id: "a",
            value: {
              type: "branchall",
              parallel: true,
              branches: [
                {
                  modules: [
                    {
                      id: "b",
                      value: {
                        input_transforms: {},
                        language: "bun" as api.RawScript.language,
                        type: "rawscript",
                        content: "export function main() { return 1; }",
                      },
                    },
                  ],
                },
                {
                  modules: [
                    {
                      id: "c",
                      value: {
                        input_transforms: {},
                        language: "bun" as api.RawScript.language,
                        type: "rawscript",
                        content: "export function main() { return 2; }",
                      },
                    },
                  ],
                },
              ],
            },
          },
        ],
      },
    };
  } else if (flowPattern == "flow_seq_3_bun") {
    return {
      path: "flow_seq_3_bun",
      args: {},
      value: {
        modules: [
          {
            id: "a",
            value: {
              input_transforms: {},
              language: "bun" as api.RawScript.language,
              type: "rawscript",
              content: "export function main() { return 1; }",
            },
          },
          {
            id: "b",
            value: {
              input_transforms: {},
              language: "bun" as api.RawScript.language,
              type: "rawscript",
              content: "export function main() { return 2; }",
            },
          },
          {
            id: "c",
            value: {
              input_transforms: {},
              language: "bun" as api.RawScript.language,
              type: "rawscript",
              content: "export function main() { return 3; }",
            },
          },
        ],
      },
    };
  } else {
    return {
      path: "2steps",
      args: {},
      value: {
        modules: [
          {
            id: "a",
            value: {
              input_transforms: {},
              language: api.RawScript.language.DENO,
              type: "rawscript",
              content:
                'export function main(){ return Deno.env.get("WM_FLOW_JOB_ID"); }',
            },
          },
          {
            id: "b",
            value: {
              type: "identity",
            },
          },
        ],
      },
    };
  }
};
