
import { assertEquals } from "jsr:@std/assert";
import type { FlowPreview } from "../../../cli/gen/index.ts";
import * as api from "../../../cli/gen/index.ts";
import { awaitJobCompletion, setup } from "./utils.ts";

const flowPreview: FlowPreview = {
  "args": {
    "a": 4,
    "b": 9
  },
  "value": {
    "modules": [
      {
        "id": "a",
        "value": {
          "type": "rawscript",
          "content": "// import * as wmill from \"windmill-client\"\n\nexport async function main(a: number, b : number) {\n  return a + b\n}\n",
          "language": "bun",
          "input_transforms": {
            "a": {
              "type": "javascript",
              "expr": "flow_input.a"
            },
            "b": {
              "type": "javascript",
              "expr": "flow_input.b"
            }
          },
          "tag": ""
        }
      },
      {
        "id": "b",
        "value": {
          "type": "rawscript",
          "content": "// import * as wmill from \"windmill-client\"\n\nexport async function main(n: number) {\n  return n / 2.0\n}\n",
          "language": "bun",
          "input_transforms": {
            "n": {
              "type": "javascript",
              "expr": "results.a"
            }
          },
          "tag": ""
        }
      }
    ]
  },
  "path": "u/admin/beauteous_flow"
};

Deno.test("Run a flow preview and check its completion", async () => {
  const { workspace } = await setup()

  const id = await api.runFlowPreview({
    workspace,
    requestBody: flowPreview
  });

  const result = await awaitJobCompletion({ id, workspace, timeoutMs: 2000 });
  assertEquals(result.result, 6.5);
  assertEquals(result.canceled, false);
  assertEquals(result.success, true);

});

Deno.test("Run a flow preview - Immediately cancel and check cancellation status", async () => {
  const { workspace } = await setup()

  const id = await api.runFlowPreview({
    workspace,
    requestBody: flowPreview
  });

  await api.cancelQueuedJob({ id, workspace, requestBody: { reason: "test cancellation"}});
  const result = await awaitJobCompletion({ id, workspace, timeoutMs: 200});
  assertEquals(result.canceled, true);
  assertEquals(result.success, false);
});