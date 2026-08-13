#!/usr/bin/env node

// Generates the AJV-ready JSON schemas in src/gen from the repo's OpenAPI specs.
// Kept in plain node (no shell) so it runs identically on Windows, where the CLI
// builds and tests this package as a local dependency.

const fs = require("fs");
const os = require("os");
const path = require("path");
// The package's own runtime YAML parser, so generating schemas needs no extra dependency.
const { parseWithPointers } = require("@stoplight/yaml");

const { generateSchemas } = require("./generate-resource-schemas.js");

// Severity 0 is DiagnosticSeverity.Error.
const YAML_ERROR = 0;

// parseWithPointers recovers from syntax errors rather than throwing, so an unchecked parse
// can hand back a spec silently missing whatever it failed to read — schemas that drop a
// definition and lint that quietly stops validating it.
function parseSpec(filePath) {
  const { data, diagnostics } = parseWithPointers(
    fs.readFileSync(filePath, "utf8")
  );
  const errors = diagnostics.filter((d) => d.severity === YAML_ERROR);
  if (errors.length > 0) {
    throw new Error(
      `${filePath} is not valid YAML:\n` +
        errors
          .map((d) => `  line ${d.range.start.line + 1}: ${d.message}`)
          .join("\n")
    );
  }
  return data;
}

const packageDir = path.join(__dirname, "..");
const repoDir = path.join(packageDir, "..");
const outputDir = path.join(packageDir, "src", "gen");

function removeDiscriminatorMappings(schema) {
  const removeMapping = (obj) => {
    if (obj && typeof obj === "object") {
      if (obj.discriminator?.mapping) delete obj.discriminator.mapping;
      for (const v of Object.values(obj)) removeMapping(v);
    }
  };
  removeMapping(schema);

  // ToolValue's discriminator does not work with the allOf in FlowModuleTool
  if (schema.components?.schemas?.ToolValue?.discriminator) {
    delete schema.components.schemas.ToolValue.discriminator;
  }
}

// AJV does not handle OpenAPI 3.0 `nullable: true` combined with `enum` — null must
// be explicitly listed in the enum for validation to accept null values.
function addNullToNullableEnums(obj) {
  if (!obj || typeof obj !== "object") return;
  if (Array.isArray(obj)) {
    obj.forEach(addNullToNullableEnums);
    return;
  }
  if (
    obj.nullable === true &&
    Array.isArray(obj.enum) &&
    !obj.enum.includes(null)
  ) {
    obj.enum.push(null);
  }
  for (const v of Object.values(obj)) addNullToNullableEnums(v);
}

function main() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "wmill-yaml-validator-"));
  try {
    fs.mkdirSync(path.join(outputDir, "triggers"), { recursive: true });

    const openflowOutputPath = path.join(outputDir, "openflow.json");
    const backendOutputPath = path.join(tmpDir, "backend-openapi.json");

    const openflowSchema = parseSpec(
      path.join(repoDir, "openflow.openapi.yaml")
    );
    const backendSchema = parseSpec(
      path.join(repoDir, "backend", "windmill-api", "openapi.yaml")
    );

    removeDiscriminatorMappings(openflowSchema);

    fs.writeFileSync(
      openflowOutputPath,
      JSON.stringify(openflowSchema, null, 2) + "\n"
    );
    fs.writeFileSync(
      backendOutputPath,
      JSON.stringify(backendSchema, null, 2) + "\n"
    );

    generateSchemas(backendOutputPath, openflowOutputPath, outputDir);

    const files = [
      openflowOutputPath,
      path.join(outputDir, "schedule.json"),
      ...fs
        .readdirSync(path.join(outputDir, "triggers"))
        .map((f) => path.join(outputDir, "triggers", f)),
    ].filter((f) => f.endsWith(".json"));

    for (const file of files) {
      const schema = JSON.parse(fs.readFileSync(file, "utf8"));
      addNullToNullableEnums(schema);
      fs.writeFileSync(file, JSON.stringify(schema, null, 2) + "\n");
    }

    console.log(`Generated ${files.length} schemas in ${outputDir}`);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

main();
