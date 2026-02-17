import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";
import * as windmill from "https://deno.land/x/windmill@v1.174.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.174.0/windmill-api/index.ts";

export const VERSION = "v1.638.3";

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
    } catch (err) { }
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
  workspace: string
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
      'export async function main(){ return (await fetch(BASE_URL + "/api/version")).text() }';
    language = "nativets";
  } else {
    throw new Error(
      "Could not create script for script pattern " + scriptPattern
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
      dedicated_worker: scriptPattern === "dedicated",
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

  if (scriptPattern === "dedicated") {
    await waitForDedicatedWorker(workspace, path);
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
              content: "# let's bloat that bash script, 3.. 2.. 1.. BOOM\n".repeat(100) + `if [[ -z $\{WM_FLOW_JOB_ID+x\} ]]; then\necho "not set"\nelif [[ -z "$WM_FLOW_JOB_ID" ]]; then\necho "empty"\nelse\necho "$WM_FLOW_JOB_ID"\nfi`,
            },
          }
        ],
      }
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
