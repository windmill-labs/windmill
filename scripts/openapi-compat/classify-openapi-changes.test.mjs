import assert from "node:assert/strict";
import { test } from "node:test";
import { classifyOpenApiChanges } from "./classify-openapi-changes.mjs";

function spec(operation) {
  return {
    openapi: "3.0.0",
    paths: {
      "/w/{workspace}/flow_conversations/{conversation_id}/messages": {
        get: operation,
      },
    },
    components: {
      schemas: {
        FlowConversationMessage: {
          type: "object",
          required: ["id"],
          properties: {
            id: { type: "string" },
            status: { type: "string", enum: ["queued", "running", "done"] },
          },
        },
      },
    },
  };
}

test("detects removed query parameters", () => {
  const before = spec({
    parameters: [
      { name: "workspace", in: "path", required: true },
      { name: "after_id", in: "query", required: false },
    ],
    responses: { 200: { description: "ok" } },
  });
  const after = spec({
    parameters: [{ name: "workspace", in: "path", required: true }],
    responses: { 200: { description: "ok" } },
  });

  const result = classifyOpenApiChanges(before, after);

  assert.equal(result.breaking, 1);
  assert.equal(result.findings[0].kind, "parameter_removed");
  assert.match(result.findings[0].message, /after_id/);
});

test("detects newly required request body fields", () => {
  const before = spec({
    requestBody: {
      content: {
        "application/json": {
          schema: {
            type: "object",
            required: ["path"],
            properties: {
              path: { type: "string" },
              args: { type: "object" },
            },
          },
        },
      },
    },
    responses: { 200: { description: "ok" } },
  });
  const after = spec({
    requestBody: {
      content: {
        "application/json": {
          schema: {
            type: "object",
            required: ["path", "args"],
            properties: {
              path: { type: "string" },
              args: { type: "object" },
            },
          },
        },
      },
    },
    responses: { 200: { description: "ok" } },
  });

  const result = classifyOpenApiChanges(before, after);

  assert.equal(result.breaking, 1);
  assert.equal(result.findings[0].kind, "request_required_field_added");
  assert.match(result.findings[0].location, /args/);
});

test("checks every matching request body media type", () => {
  const before = spec({
    requestBody: {
      content: {
        "application/json": {
          schema: {
            type: "object",
            required: ["path"],
            properties: { path: { type: "string" }, token: { type: "string" } },
          },
        },
        "application/x-www-form-urlencoded": {
          schema: {
            type: "object",
            required: ["path"],
            properties: { path: { type: "string" }, token: { type: "string" } },
          },
        },
      },
    },
    responses: { 200: { description: "ok" } },
  });
  const after = structuredClone(before);
  after.paths["/w/{workspace}/flow_conversations/{conversation_id}/messages"].get.requestBody.content[
    "application/x-www-form-urlencoded"
  ].schema.required.push("token");

  const result = classifyOpenApiChanges(before, after);

  assert.equal(result.breaking, 1);
  assert.equal(result.findings[0].kind, "request_required_field_added");
  assert.match(result.findings[0].location, /application~1x-www-form-urlencoded/);
});

test("classifies response required fields as warnings by default", () => {
  const before = spec({
    responses: {
      200: {
        description: "ok",
        content: {
          "application/json": {
            schema: {
              type: "object",
              required: ["id"],
              properties: { id: { type: "string" }, created_seq: { type: "number" } },
            },
          },
        },
      },
    },
  });
  const after = spec({
    responses: {
      200: {
        description: "ok",
        content: {
          "application/json": {
            schema: {
              type: "object",
              required: ["id", "created_seq"],
              properties: { id: { type: "string" }, created_seq: { type: "number" } },
            },
          },
        },
      },
    },
  });

  const result = classifyOpenApiChanges(before, after);

  assert.equal(result.breaking, 0);
  assert.equal(result.warnings, 1);
  assert.equal(result.findings[0].kind, "response_required_field_added");
});

test("can treat response required fields as breaking in strict mode", () => {
  const before = {
    openapi: "3.0.0",
    paths: {},
    components: {
      schemas: {
        UsedTriggers: {
          type: "object",
          required: ["email"],
          properties: { email: { type: "string" }, azure_used: { type: "number" } },
        },
      },
    },
  };
  const after = {
    openapi: "3.0.0",
    paths: {},
    components: {
      schemas: {
        UsedTriggers: {
          type: "object",
          required: ["email", "azure_used"],
          properties: { email: { type: "string" }, azure_used: { type: "number" } },
        },
      },
    },
  };

  const result = classifyOpenApiChanges(before, after, { responseRequiredSeverity: "breaking" });

  assert.equal(result.breaking, 1);
  assert.equal(result.findings[0].kind, "schema_required_field_added");
});

test("detects enum narrowing", () => {
  const before = spec({ responses: { 200: { description: "ok" } } });
  const after = structuredClone(before);
  after.components.schemas.FlowConversationMessage.properties.status.enum = ["running", "done"];

  const result = classifyOpenApiChanges(before, after);

  assert.equal(result.breaking, 1);
  assert.equal(result.findings[0].kind, "enum_values_removed");
  assert.deepEqual(result.findings[0].before, ["queued", "running", "done"]);
});
