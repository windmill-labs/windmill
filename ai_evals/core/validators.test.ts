import { describe, expect, it } from "bun:test";
import { validateScriptState } from "./validators";

describe("validateScriptState", () => {
  it("accepts semantically equivalent script implementations", () => {
    const checks = validateScriptState({
      actual: {
        path: "f/evals/greet_user.ts",
        lang: "bun",
        code: "export async function main(name: string): Promise<string> {\n  return `Hello, ${name}!`;\n}\n",
      },
      expected: {
        path: "f/evals/greet_user.ts",
        lang: "bun",
        code: "export async function main(name: string) {\n\treturn `Hello, ${name}!`\n}\n",
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("still requires an exported main entrypoint", () => {
    const checks = validateScriptState({
      actual: {
        path: "f/evals/greet_user.ts",
        lang: "bun",
        code: "async function main(name: string) {\n  return `Hello, ${name}!`;\n}\n",
      },
    });

    expect(checks).toContainEqual({
      name: "script exports entrypoint",
      passed: false,
    });
  });
});
