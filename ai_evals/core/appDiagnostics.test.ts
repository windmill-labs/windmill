import { describe, expect, it } from "bun:test";
import { fileURLToPath } from "node:url";
import { loadAppFixture } from "../adapters/frontend/core/app/appFixtureLoader";
import { buildAppWmillTypes, collectAppDiagnostics } from "./appDiagnostics";

const FILE_MANAGER_FIXTURE = fileURLToPath(
  new URL("../fixtures/frontend/app/initial/file_manager", import.meta.url)
);

describe("collectAppDiagnostics", () => {
  it("accepts seeded multi-file apps without static analysis errors", async () => {
    const fixture = await loadAppFixture(FILE_MANAGER_FIXTURE);
    const diagnostics = collectAppDiagnostics({
      frontend: fixture.frontend,
      backend: fixture.backend,
    });

    expect(diagnostics.lintResult.errorCount).toBe(0);
  });

  it("reports missing backend references through the generated wmill typings", () => {
    const diagnostics = collectAppDiagnostics({
      frontend: {
        "/index.tsx":
          "import { backend } from 'wmill'\nexport default function App() { void backend.deleteRecipe({ id: 1 }); return <div /> }\n",
      },
      backend: {
        listRecipes: {
          name: "List recipes",
          type: "inline",
          inlineScript: {
            language: "bun",
            content: "export async function main() { return [] }\n",
          },
        },
      },
    });

    expect(diagnostics.lintResult.errorCount).toBeGreaterThan(0);
    expect(diagnostics.lintResult.errors.frontend["/index.tsx"]?.join("\n")).toContain(
      "Property 'deleteRecipe' does not exist"
    );
  });

  it("reports backend argument shape mismatches when the inline main signature is portable", () => {
    const diagnostics = collectAppDiagnostics({
      frontend: {
        "/index.tsx":
          "import { backend } from 'wmill'\nexport default function App() { void backend.addRecipe({ name: 'Soup' }); return <div /> }\n",
      },
      backend: {
        addRecipe: {
          name: "Add recipe",
          type: "inline",
          inlineScript: {
            language: "bun",
            content:
              "export async function main({ name, ingredients }: { name: string; ingredients: string }) { return { name, ingredients } }\n",
          },
        },
      },
    });

    expect(diagnostics.lintResult.errorCount).toBeGreaterThan(0);
    expect(diagnostics.lintResult.errors.frontend["/index.tsx"]?.join("\n")).toContain(
      "Property 'ingredients' is missing"
    );
  });
});

describe("buildAppWmillTypes", () => {
  it("generates callable signatures for zero-arg and typed runnables", () => {
    const wmillTypes = buildAppWmillTypes({
      listRecipes: {
        name: "List recipes",
        type: "inline",
        inlineScript: {
          language: "bun",
          content: "export async function main() { return [] }\n",
        },
      },
      addRecipe: {
        name: "Add recipe",
        type: "inline",
        inlineScript: {
          language: "bun",
          content:
            "export async function main({ name }: { name: string }) { return { name } }\n",
        },
      },
    });

    expect(wmillTypes).toContain('"listRecipes": () => Promise<any>;');
    expect(wmillTypes).toContain('"addRecipe": (args: { name: string }) => Promise<any>;');
  });
});
