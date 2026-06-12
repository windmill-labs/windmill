#!/usr/bin/env node

import { readFile } from "node:fs/promises";

const HTTP_METHODS = new Set(["get", "put", "post", "delete", "patch", "options", "head", "trace"]);

export function pointer(parts) {
  return parts
    .map((part) =>
      String(part)
        .replaceAll("~", "~0")
        .replaceAll("/", "~1"),
    )
    .join("/");
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function asArray(value) {
  return Array.isArray(value) ? value : [];
}

function get(obj, parts) {
  return parts.reduce((current, part) => (current === undefined || current === null ? undefined : current[part]), obj);
}

function collectOperations(spec) {
  const operations = new Map();
  for (const [pathName, pathItem] of Object.entries(spec.paths ?? {})) {
    for (const [method, operation] of Object.entries(pathItem ?? {})) {
      if (!HTTP_METHODS.has(method)) continue;
      operations.set(`${method.toUpperCase()} ${pathName}`, {
        pathName,
        method,
        pathItem,
        operation,
      });
    }
  }
  return operations;
}

function collectParameters(pathItem, operation) {
  const parameters = new Map();
  for (const parameter of [...asArray(pathItem?.parameters), ...asArray(operation?.parameters)]) {
    if (!parameter || parameter.$ref) continue;
    parameters.set(`${parameter.in}:${parameter.name}`, parameter);
  }
  return parameters;
}

function collectRequiredSchemaProperties(schema, basePath = []) {
  const required = [];
  if (!isObject(schema)) return required;

  for (const field of asArray(schema.required)) {
    required.push([...basePath, field]);
  }

  for (const [name, child] of Object.entries(schema.properties ?? {})) {
    required.push(...collectRequiredSchemaProperties(child, [...basePath, name]));
  }

  if (isObject(schema.items)) {
    required.push(...collectRequiredSchemaProperties(schema.items, [...basePath, "*"]));
  }

  return required;
}

function collectEnums(schema, basePath = []) {
  const enums = [];
  if (!isObject(schema)) return enums;

  if (Array.isArray(schema.enum)) {
    enums.push({ path: basePath, values: schema.enum.map(String) });
  }

  for (const [name, child] of Object.entries(schema.properties ?? {})) {
    enums.push(...collectEnums(child, [...basePath, name]));
  }

  if (isObject(schema.items)) {
    enums.push(...collectEnums(schema.items, [...basePath, "*"]));
  }

  return enums;
}

function jsonPointer(path) {
  return `/${pointer(path)}`;
}

function makeFinding({ severity, kind, location, message, before, after }) {
  return { severity, kind, location, message, before, after };
}

function requestBodySchemas(operation) {
  const body = operation?.requestBody;
  const content = body?.content ?? {};
  const schemas = [];
  for (const [mediaType, media] of Object.entries(content)) {
    if (media?.schema) schemas.push({ mediaType, schema: media.schema });
  }
  return schemas;
}

function responseSchemas(operation) {
  const schemas = [];
  for (const [status, response] of Object.entries(operation?.responses ?? {})) {
    const schema = response?.content?.["application/json"]?.schema;
    if (schema) schemas.push({ status, schema });
  }
  return schemas;
}

function diffRequestBodySchemas({ beforeOperation, afterOperation, operationLocation, options }) {
  const findings = [];
  const beforeSchemas = new Map(requestBodySchemas(beforeOperation).map((entry) => [entry.mediaType, entry.schema]));
  const afterSchemas = new Map(requestBodySchemas(afterOperation).map((entry) => [entry.mediaType, entry.schema]));

  for (const [mediaType, beforeSchema] of beforeSchemas) {
    const afterSchema = afterSchemas.get(mediaType);
    if (!afterSchema) continue;
    const location = `${operationLocation}/requestBody/content/${pointer([mediaType])}`;
    findings.push(
      ...diffRequiredProperties({
        beforeSchema,
        afterSchema,
        location,
        responseRequiredSeverity: options.responseRequiredSeverity,
        schemaRequiredSeverity: options.schemaRequiredSeverity,
      }),
    );
    findings.push(...diffEnums({ beforeSchema, afterSchema, location }));
  }

  return findings;
}

function diffRequiredProperties({ beforeSchema, afterSchema, location, responseRequiredSeverity, schemaRequiredSeverity }) {
  const findings = [];
  const beforeRequired = new Set(collectRequiredSchemaProperties(beforeSchema).map(jsonPointer));
  const afterRequired = new Set(collectRequiredSchemaProperties(afterSchema).map(jsonPointer));

  for (const requiredPath of afterRequired) {
    if (!beforeRequired.has(requiredPath)) {
      const isResponse = location.includes("/responses/");
      const isComponent = location.includes("/components/schemas/");
      findings.push(
        makeFinding({
          severity: isResponse ? responseRequiredSeverity : isComponent ? schemaRequiredSeverity : "breaking",
          kind: isResponse
            ? "response_required_field_added"
            : isComponent
              ? "schema_required_field_added"
              : "request_required_field_added",
          location: `${location}${requiredPath}`,
          message: `Required schema field was added at ${requiredPath}`,
        }),
      );
    }
  }

  return findings;
}

function diffEnums({ beforeSchema, afterSchema, location }) {
  const findings = [];
  const beforeEnums = new Map(collectEnums(beforeSchema).map((entry) => [jsonPointer(entry.path), entry.values]));
  const afterEnums = new Map(collectEnums(afterSchema).map((entry) => [jsonPointer(entry.path), entry.values]));

  for (const [enumPath, beforeValues] of beforeEnums) {
    const afterValues = afterEnums.get(enumPath);
    if (!afterValues) continue;
    const removed = beforeValues.filter((value) => !afterValues.includes(value));
    if (removed.length > 0) {
      findings.push(
        makeFinding({
          severity: "breaking",
          kind: "enum_values_removed",
          location: `${location}${enumPath}`,
          message: `Enum values removed: ${removed.join(", ")}`,
          before: beforeValues,
          after: afterValues,
        }),
      );
    }
  }

  return findings;
}

function diffOperations(before, after, options) {
  const findings = [];
  const beforeOperations = collectOperations(before);
  const afterOperations = collectOperations(after);

  for (const [operationKey, beforeEntry] of beforeOperations) {
    const afterEntry = afterOperations.get(operationKey);
    const operationLocation = `/paths/${pointer([beforeEntry.pathName])}/${beforeEntry.method}`;

    if (!afterEntry) {
      findings.push(
        makeFinding({
          severity: "breaking",
          kind: "operation_removed",
          location: operationLocation,
          message: `${operationKey} was removed`,
        }),
      );
      continue;
    }

    const beforeParameters = collectParameters(beforeEntry.pathItem, beforeEntry.operation);
    const afterParameters = collectParameters(afterEntry.pathItem, afterEntry.operation);
    for (const [parameterKey, beforeParameter] of beforeParameters) {
      const afterParameter = afterParameters.get(parameterKey);
      if (!afterParameter) {
        findings.push(
          makeFinding({
            severity: "breaking",
            kind: "parameter_removed",
            location: `${operationLocation}/parameters/${parameterKey}`,
            message: `${operationKey} parameter ${parameterKey} was removed`,
          }),
        );
      } else if (!beforeParameter.required && afterParameter.required) {
        findings.push(
          makeFinding({
            severity: "breaking",
            kind: "parameter_became_required",
            location: `${operationLocation}/parameters/${parameterKey}`,
            message: `${operationKey} parameter ${parameterKey} became required`,
          }),
        );
      }
    }

    for (const [parameterKey, afterParameter] of afterParameters) {
      if (!beforeParameters.has(parameterKey) && afterParameter.required) {
        findings.push(
          makeFinding({
            severity: "breaking",
            kind: "required_parameter_added",
            location: `${operationLocation}/parameters/${parameterKey}`,
            message: `${operationKey} added required parameter ${parameterKey}`,
          }),
        );
      }
    }

    const beforeRequestBody = beforeEntry.operation?.requestBody;
    const afterRequestBody = afterEntry.operation?.requestBody;
    if (!beforeRequestBody?.required && afterRequestBody?.required) {
      findings.push(
        makeFinding({
          severity: "breaking",
          kind: "request_body_became_required",
          location: `${operationLocation}/requestBody`,
          message: `${operationKey} request body became required`,
        }),
      );
    }

    findings.push(
      ...diffRequestBodySchemas({
        beforeOperation: beforeEntry.operation,
        afterOperation: afterEntry.operation,
        operationLocation,
        options,
      }),
    );

    const beforeResponses = new Map(responseSchemas(beforeEntry.operation).map((entry) => [entry.status, entry.schema]));
    const afterResponses = new Map(responseSchemas(afterEntry.operation).map((entry) => [entry.status, entry.schema]));
    for (const [status, beforeSchema] of beforeResponses) {
      const afterSchema = afterResponses.get(status);
      if (!afterSchema) continue;
      findings.push(
        ...diffRequiredProperties({
          beforeSchema,
          afterSchema,
          location: `${operationLocation}/responses/${status}`,
          responseRequiredSeverity: options.responseRequiredSeverity,
          schemaRequiredSeverity: options.schemaRequiredSeverity,
        }),
      );
      findings.push(...diffEnums({ beforeSchema, afterSchema, location: `${operationLocation}/responses/${status}` }));
    }
  }

  return findings;
}

function diffComponentSchemas(before, after, options) {
  const findings = [];
  const beforeSchemas = before.components?.schemas ?? {};
  const afterSchemas = after.components?.schemas ?? {};

  for (const [schemaName, beforeSchema] of Object.entries(beforeSchemas)) {
    const afterSchema = afterSchemas[schemaName];
    if (!afterSchema) {
      findings.push(
        makeFinding({
          severity: "breaking",
          kind: "schema_removed",
          location: `/components/schemas/${pointer([schemaName])}`,
          message: `Schema ${schemaName} was removed`,
        }),
      );
      continue;
    }

    findings.push(
      ...diffRequiredProperties({
        beforeSchema,
        afterSchema,
        location: `/components/schemas/${pointer([schemaName])}`,
        responseRequiredSeverity: options.responseRequiredSeverity,
        schemaRequiredSeverity: options.schemaRequiredSeverity,
      }),
    );
    findings.push(...diffEnums({ beforeSchema, afterSchema, location: `/components/schemas/${pointer([schemaName])}` }));
  }

  return findings;
}

export function classifyOpenApiChanges(before, after, options = {}) {
  const normalizedOptions = {
    responseRequiredSeverity: options.responseRequiredSeverity ?? "warning",
    schemaRequiredSeverity: options.schemaRequiredSeverity ?? options.responseRequiredSeverity ?? "warning",
  };
  const findings = [
    ...diffOperations(before, after, normalizedOptions),
    ...diffComponentSchemas(before, after, normalizedOptions),
  ];

  const counts = findings.reduce(
    (acc, finding) => {
      acc[finding.severity] = (acc[finding.severity] ?? 0) + 1;
      return acc;
    },
    { breaking: 0, warning: 0 },
  );

  return {
    breaking: counts.breaking,
    warnings: counts.warning,
    findings,
  };
}

async function main() {
  const args = process.argv.slice(2);
  const strictResponse = args.includes("--response-required=breaking");
  const positional = args.filter((arg) => !arg.startsWith("--"));

  if (positional.length !== 2) {
    console.error(
      "Usage: classify-openapi-changes.mjs <before.json> <after.json> [--response-required=breaking]",
    );
    process.exitCode = 2;
    return;
  }

  const before = JSON.parse(await readFile(positional[0], "utf8"));
  const after = JSON.parse(await readFile(positional[1], "utf8"));
  const result = classifyOpenApiChanges(before, after, {
    responseRequiredSeverity: strictResponse ? "breaking" : "warning",
  });

  console.log(JSON.stringify(result, null, 2));
  if (result.breaking > 0) {
    process.exitCode = 1;
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  await main();
}
