import { WindmillYamlValidator } from "../yaml-validator";
import * as fs from "fs";
import * as path from "path";

describe("WindmillYamlValidator", () => {
  let validator: WindmillYamlValidator;
  const samplesDir = path.join(__dirname, "test-samples");

  const readSample = (filename: string): string => {
    return fs.readFileSync(path.join(samplesDir, filename), "utf-8");
  };

  beforeEach(() => {
    validator = new WindmillYamlValidator();
  });

  describe("constructor", () => {
    it("should create a validator instance", () => {
      expect(validator).toBeInstanceOf(WindmillYamlValidator);
    });

    it("should initialize without throwing", () => {
      expect(() => new WindmillYamlValidator()).not.toThrow();
    });
  });

  describe("validate (flow target)", () => {
    it("should throw error for non-string input", () => {
      expect(() => validator.validate(null as any, { type: "flow" })).toThrow(
        "Document must be a string"
      );
      expect(() => validator.validate(123 as any, { type: "flow" })).toThrow(
        "Document must be a string"
      );
      expect(() => validator.validate({} as any, { type: "flow" })).toThrow(
        "Document must be a string"
      );
      expect(() => validator.validate([] as any, { type: "flow" })).toThrow(
        "Document must be a string"
      );
    });

    describe("valid flows", () => {
      it("should validate a valid minimal flow from sample file", () => {
        const validFlow = readSample("valid-minimal.yaml");

        const result = validator.validate(validFlow, { type: "flow" });

        expect(result.errors).toHaveLength(0);
        expect(result.parsed).toBeDefined();
        expect(result.parsed.data).toMatchObject({
          summary: "Test Flow",
          value: {
            modules: [],
          },
        });
      });

      it("should validate a script flow from sample file", () => {
        const validFlow = readSample("valid-script-flow.yaml");

        const result = validator.validate(validFlow, { type: "flow" });

        expect(result.errors).toHaveLength(0);
        expect(result.parsed.data).toMatchObject({
          summary: "Simple Script Flow",
          description: "A basic flow that runs a TypeScript script",
          schema: expect.objectContaining({
            type: "object",
            properties: expect.objectContaining({
              message: expect.objectContaining({
                type: "string",
                default: "Hello World",
              }),
            }),
          }),
          value: {
            modules: expect.arrayContaining([
              expect.objectContaining({
                id: "script_step",
                value: expect.objectContaining({
                  type: "rawscript",
                  language: "deno",
                  input_transforms: {},
                }),
              }),
            ]),
          },
        });
      });
    });

    describe("invalid flows", () => {
      it("should return errors for missing summary from sample file", () => {
        const invalidFlow = readSample("invalid-missing-summary.yaml");

        const result = validator.validate(invalidFlow, { type: "flow" });

        expect(result.errors.length).toBeGreaterThan(0);
        expect(
          result.errors.some(
            (error) =>
              error.instancePath === "" &&
              error.keyword === "required" &&
              error.params?.missingProperty === "summary"
          )
        ).toBe(true);
      });

      it("should return errors for invalid types from sample file", () => {
        const invalidFlow = readSample("invalid-wrong-types.yaml");

        const result = validator.validate(invalidFlow, { type: "flow" });

        expect(result.errors.length).toBeGreaterThan(0);
        expect(
          result.errors.some(
            (error) =>
              error.instancePath === "/summary" && error.keyword === "type"
          )
        ).toBe(true);
      });

      it("should return errors for invalid language from sample file", () => {
        const invalidFlow = readSample("invalid-language.yaml");

        const result = validator.validate(invalidFlow, { type: "flow" });

        expect(result.errors.length).toBeGreaterThan(0);
        expect(
          result.errors.some(
            (error) =>
              error.instancePath === "/value/modules/0/value/language" &&
              error.keyword === "enum"
          )
        ).toBe(true);
      });

      it("should handle empty file from sample", () => {
        const emptyFlow = readSample("empty.yaml");

        const result = validator.validate(emptyFlow, { type: "flow" });

        expect(result.parsed).toBeDefined();
        expect(result.errors.length).toBeGreaterThan(0);
      });

      it("should handle complex invalid flow with comprehensive error detection", () => {
        const complexInvalidFlow = readSample("invalid-complex-flow.yaml");

        const result = validator.validate(complexInvalidFlow, { type: "flow" });

        expect(result.errors.length).toBeGreaterThan(20); // Should have many errors

        // Check for missing required fields errors
        const missingRequiredErrors = result.errors.filter(
          (error) => error.keyword === "required"
        );
        expect(missingRequiredErrors.length).toBeGreaterThan(10);

        // Check for invalid enum errors (invalid language)
        const enumErrors = result.errors.filter(
          (error) => error.keyword === "enum"
        );
        expect(enumErrors.length).toBeGreaterThan(0);

        // Check for type errors (string vs number, boolean vs string, etc.)
        const typeErrors = result.errors.filter(
          (error) => error.keyword === "type"
        );
        expect(typeErrors.length).toBeGreaterThan(5);

        // Should detect invalid forloop structure (by checking for forloop-related required fields)
        const forloopRequiredErrors = result.errors.filter(
          (error) =>
            error.keyword === "required" &&
            error.message &&
            (error.message.includes("modules") ||
              error.message.includes("iterator") ||
              error.message.includes("skip_failures"))
        );
        expect(forloopRequiredErrors.length).toBeGreaterThan(0);

        // Should detect invalid branch structure
        const branchRequiredErrors = result.errors.filter(
          (error) =>
            error.keyword === "required" &&
            error.message &&
            (error.message.includes("branches") ||
              error.message.includes("default") ||
              error.message.includes("expr"))
        );
        expect(branchRequiredErrors.length).toBeGreaterThan(0);

        // Should detect discriminator errors for invalid transform types
        const discriminatorErrors = result.errors.filter(
          (error) => error.keyword === "discriminator"
        );
        expect(discriminatorErrors.length).toBeGreaterThan(0);
      });

      it("should handle deeply nested invalid structures with detailed error reporting", () => {
        const nestedInvalidFlow = readSample("invalid-nested-structures.yaml");

        const result = validator.validate(nestedInvalidFlow, { type: "flow" });

        expect(result.errors.length).toBeGreaterThan(10); // Should have many nested errors

        // Check for deeply nested path errors (paths with many levels)
        const deepNestedErrors = result.errors.filter(
          (error) => error.instancePath.split("/").length > 6 // Deep nesting
        );
        expect(deepNestedErrors.length).toBeGreaterThan(0);

        // Check for transform-related errors
        const transformErrors = result.errors.filter(
          (error) =>
            error.instancePath.includes("input_transforms") ||
            error.instancePath.includes("iterator")
        );
        expect(transformErrors.length).toBeGreaterThan(0);

        // Check for type errors in general
        const typeErrors = result.errors.filter(
          (error) => error.keyword === "type"
        );
        expect(typeErrors.length).toBeGreaterThan(0);

        // Check for missing required fields in nested structures
        const nestedRequiredErrors = result.errors.filter(
          (error) =>
            error.keyword === "required" &&
            error.instancePath.includes("/modules/")
        );
        expect(nestedRequiredErrors.length).toBeGreaterThan(0);

        // Check for discriminator errors in nested transforms
        const nestedDiscriminatorErrors = result.errors.filter(
          (error) => error.keyword === "discriminator"
        );
        expect(nestedDiscriminatorErrors.length).toBeGreaterThan(0);
      });

      it("should provide specific error locations for complex validation failures", () => {
        const complexInvalidFlow = readSample("invalid-complex-flow.yaml");

        const result = validator.validate(complexInvalidFlow, { type: "flow" });

        // Verify that errors have meaningful instance paths
        const errorsWithPaths = result.errors.filter(
          (error) => error.instancePath && error.instancePath.length > 0
        );
        expect(errorsWithPaths.length).toBeGreaterThan(5);

        // Check that we can identify specific problematic modules
        const moduleSpecificErrors = result.errors.filter((error) =>
          error.instancePath.includes("/value/modules/")
        );
        expect(moduleSpecificErrors.length).toBeGreaterThan(0);

        // Verify error messages are descriptive
        const descriptiveErrors = result.errors.filter(
          (error) => error.message && error.message.length > 0
        );
        expect(descriptiveErrors.length).toBe(result.errors.length);
      });

      it("should handle all major flow control structures with validation errors", () => {
        const complexInvalidFlow = readSample("invalid-complex-flow.yaml");

        const result = validator.validate(complexInvalidFlow, { type: "flow" });

        expect(result.errors.length).toBeGreaterThan(20);

        // Should find errors related to flow control structures by checking instance paths
        const flowControlErrors = result.errors.filter(
          (error) =>
            error.instancePath.includes("/modules/2/") || // forloop module
            error.instancePath.includes("/modules/3/") || // forloop module
            error.instancePath.includes("/modules/4/") || // branch module
            error.instancePath.includes("/modules/5/") || // branch module
            error.instancePath.includes("/modules/6/") || // branch all module
            error.instancePath.includes("/modules/7/") // while loop module
        );
        expect(flowControlErrors.length).toBeGreaterThan(10);

        // Should find errors in script and flow references
        const pathReferenceErrors = result.errors.filter(
          (error) =>
            error.instancePath.includes("/modules/8/") || // script path module
            error.instancePath.includes("/modules/9/") // flow path module
        );
        expect(pathReferenceErrors.length).toBeGreaterThan(0);

        // Should detect modules with missing IDs
        const missingIdErrors = result.errors.filter(
          (error) =>
            error.keyword === "required" &&
            error.message &&
            error.message.includes("id")
        );
        expect(missingIdErrors.length).toBeGreaterThan(0);

        // Should detect type mismatches at the flow level
        const flowLevelTypeErrors = result.errors.filter(
          (error) =>
            error.instancePath.startsWith("/value/") &&
            !error.instancePath.includes("/modules/") &&
            error.keyword === "type"
        );
        expect(flowLevelTypeErrors.length).toBeGreaterThan(0);
      });
    });

    describe("AI agent flows", () => {
      describe("valid AI agent flows", () => {
        it("should validate a basic AI agent flow with FlowModule tools", () => {
          const validFlow = readSample("valid-aiagent-basic.yaml");

          const result = validator.validate(validFlow, { type: "flow" });

          expect(result.errors).toHaveLength(0);
        });

        it("should validate an AI agent flow with MCP tools", () => {
          const validFlow = readSample("valid-aiagent-mcp.yaml");

          const result = validator.validate(validFlow, { type: "flow" });

          expect(result.errors).toHaveLength(0);
        });

        it("should validate an AI agent flow with mixed FlowModule and MCP tools", () => {
          const validFlow = readSample("valid-aiagent-mixed.yaml");

          const result = validator.validate(validFlow, { type: "flow" });

          expect(result.errors).toHaveLength(0);
        });

        it("should validate an AI agent flow with parallel execution enabled", () => {
          const validFlow = readSample("valid-aiagent-parallel.yaml");

          const result = validator.validate(validFlow, { type: "flow" });

          expect(result.errors).toHaveLength(0);
          // Verify tools array has expected structure
          const agentModule = (result.parsed.data as any).value.modules[0];
          expect(agentModule.value.tools).toHaveLength(2);
        });
      });

      describe("invalid AI agent flows", () => {
        it("should return errors for AI agent missing required tools field", () => {
          const invalidFlow = readSample("invalid-aiagent-missing-tools.yaml");

          const result = validator.validate(invalidFlow, { type: "flow" });

          expect(result.errors.length).toBeGreaterThan(0);
          expect(
            result.errors.some(
              (error) =>
                error.keyword === "required" &&
                error.params?.missingProperty === "tools"
            )
          ).toBe(true);
        });

        it("should return errors for AI agent missing type field", () => {
          const invalidFlow = readSample("invalid-aiagent-missing-type.yaml");

          const result = validator.validate(invalidFlow, { type: "flow" });

          expect(result.errors.length).toBeGreaterThan(0);
          // Should fail discriminator validation since type is missing
          expect(
            result.errors.some(
              (error) =>
                error.keyword === "required" ||
                error.keyword === "discriminator"
            )
          ).toBe(true);
        });

        it("should return errors for AI agent with invalid tool_type", () => {
          const invalidFlow = readSample(
            "invalid-aiagent-invalid-tool-type.yaml"
          );

          const result = validator.validate(invalidFlow, { type: "flow" });

          expect(result.errors.length).toBeGreaterThan(0);
          // Should fail discriminator validation for invalid tool_type
          expect(
            result.errors.some(
              (error) =>
                error.keyword === "discriminator" || error.keyword === "enum"
            )
          ).toBe(true);
        });

        it("should return errors for MCP tool missing resource_path", () => {
          const invalidFlow = readSample(
            "invalid-aiagent-mcp-missing-resource.yaml"
          );

          const result = validator.validate(invalidFlow, { type: "flow" });

          expect(result.errors.length).toBeGreaterThan(0);
          expect(
            result.errors.some(
              (error) =>
                error.keyword === "required" &&
                error.params?.missingProperty === "resource_path"
            )
          ).toBe(true);
        });
      });
    });
  });
});
