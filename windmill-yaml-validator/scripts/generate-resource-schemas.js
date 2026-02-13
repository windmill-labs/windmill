#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

const CLI_EXCLUDED_FIELDS = new Set([
  "workspace_id",
  "path",
  "name",
  "versions",
  "id",
  "created_at",
  "updated_at",
  "created_by",
  "updated_by",
  "edited_at",
  "edited_by",
  "archived",
  "has_draft",
  "error",
  "last_server_ping",
  "server_id",
  "extra_perms",
  "email",
  "mode",
]);

const TARGET_SCHEMAS = {
  schedule: "Schedule",
  triggers: {
    http: "HttpTrigger",
    websocket: "WebsocketTrigger",
    kafka: "KafkaTrigger",
    nats: "NatsTrigger",
    postgres: "PostgresTrigger",
    mqtt: "MqttTrigger",
    sqs: "SqsTrigger",
    gcp: "GcpTrigger",
    email: "EmailTrigger",
  },
};

function deepClone(value) {
  return JSON.parse(JSON.stringify(value));
}

function loadJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function mergeSchemas(target, source) {
  if (!source || typeof source !== "object") {
    return;
  }

  if (source.type && !target.type) {
    target.type = source.type;
  }

  if (source.properties && typeof source.properties === "object") {
    target.properties = target.properties || {};
    Object.assign(target.properties, deepClone(source.properties));
  }

  if (Array.isArray(source.required)) {
    target.required = target.required || [];
    target.required.push(...source.required);
  }
}

function getRefName(refPath) {
  const marker = "#/components/schemas/";
  const markerIndex = refPath.indexOf(marker);
  if (markerIndex === -1) {
    return null;
  }
  return refPath.slice(markerIndex + marker.length);
}

function resolveRef(refPath, backendSchemas, openflowSchemas) {
  const refName = getRefName(refPath);
  if (!refName) {
    return null;
  }

  if (refPath.startsWith("#/components/schemas/")) {
    if (backendSchemas[refName]) {
      return deepClone(backendSchemas[refName]);
    }
    if (openflowSchemas[refName]) {
      return deepClone(openflowSchemas[refName]);
    }
    return null;
  }

  if (refPath.includes("openflow.openapi.yaml#/components/schemas/")) {
    if (openflowSchemas[refName]) {
      return deepClone(openflowSchemas[refName]);
    }
    return null;
  }

  return null;
}

function extractCliSchema(schema, allSchemas, openflowSchemas) {
  if (!schema || typeof schema !== "object") {
    return {};
  }

  const result = { type: "object", properties: {}, required: [] };

  if (Array.isArray(schema.allOf)) {
    for (const item of schema.allOf) {
      if (!item || typeof item !== "object") {
        continue;
      }
      if (item.$ref) {
        const refSchema = resolveRef(item.$ref, allSchemas, openflowSchemas);
        const transformed = extractCliSchema(refSchema, allSchemas, openflowSchemas);
        mergeSchemas(result, transformed);
      } else {
        const transformed = extractCliSchema(item, allSchemas, openflowSchemas);
        mergeSchemas(result, transformed);
      }
    }
  }

  if (schema.properties && typeof schema.properties === "object") {
    for (const [key, value] of Object.entries(schema.properties)) {
      if (CLI_EXCLUDED_FIELDS.has(key)) {
        continue;
      }
      result.properties[key] = deepClone(value);
    }
  }

  if (Array.isArray(schema.required)) {
    for (const field of schema.required) {
      if (!CLI_EXCLUDED_FIELDS.has(field)) {
        result.required.push(field);
      }
    }
  }

  const dedupRequired = Array.from(new Set(result.required));
  result.required = dedupRequired.filter((key) => key in result.properties);

  return result;
}

function resolveSchemaRefs(value, backendSchemas, openflowSchemas, stack = new Set()) {
  if (Array.isArray(value)) {
    return value.map((item) => resolveSchemaRefs(item, backendSchemas, openflowSchemas, stack));
  }

  if (!value || typeof value !== "object") {
    return value;
  }

  if (typeof value.$ref === "string") {
    const refName = getRefName(value.$ref);
    if (!refName) {
      return value;
    }

    if (stack.has(refName)) {
      return {};
    }

    const resolved = resolveRef(value.$ref, backendSchemas, openflowSchemas);
    if (!resolved) {
      return value;
    }

    const merged = { ...resolved, ...value };
    delete merged.$ref;
    const nextStack = new Set(stack);
    nextStack.add(refName);
    return resolveSchemaRefs(merged, backendSchemas, openflowSchemas, nextStack);
  }

  const result = {};
  for (const [key, nested] of Object.entries(value)) {
    result[key] = resolveSchemaRefs(nested, backendSchemas, openflowSchemas, stack);
  }
  return result;
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2) + "\n", "utf8");
}

function generateSchemas(backendPath, openflowPath, outputDir) {
  const backendOpenapi = loadJson(backendPath);
  const openflowOpenapi = loadJson(openflowPath);

  const backendSchemas = backendOpenapi.components?.schemas || {};
  const openflowSchemas = openflowOpenapi.components?.schemas || {};

  const scheduleSchema = extractCliSchema(
    backendSchemas[TARGET_SCHEMAS.schedule],
    backendSchemas,
    openflowSchemas
  );
  const resolvedScheduleSchema = resolveSchemaRefs(scheduleSchema, backendSchemas, openflowSchemas);
  writeJson(path.join(outputDir, "schedule.json"), resolvedScheduleSchema);

  for (const [triggerKind, schemaName] of Object.entries(TARGET_SCHEMAS.triggers)) {
    const triggerSchema = extractCliSchema(
      backendSchemas[schemaName],
      backendSchemas,
      openflowSchemas
    );
    const resolvedTriggerSchema = resolveSchemaRefs(triggerSchema, backendSchemas, openflowSchemas);
    writeJson(path.join(outputDir, "triggers", `${triggerKind}.json`), resolvedTriggerSchema);
  }
}

function main() {
  const [backendPath, openflowPath, outputDir] = process.argv.slice(2);
  if (!backendPath || !openflowPath || !outputDir) {
    console.error(
      "Usage: node scripts/generate-resource-schemas.js <backend-openapi.json> <openflow.json> <output-dir>"
    );
    process.exit(1);
  }

  generateSchemas(backendPath, openflowPath, outputDir);
  console.log("Generated schedule and trigger schemas");
}

main();
