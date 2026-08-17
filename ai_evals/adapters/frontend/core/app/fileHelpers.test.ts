import { describe, expect, it } from "bun:test";
import { createAppFileHelpers } from "./fileHelpers";

describe("createAppFileHelpers", () => {
  it("exposes generated wmill typings and returns real lint diagnostics", async () => {
    const { helpers, cleanup } = await createAppFileHelpers(
      {
        "/index.tsx":
          "import { backend } from 'wmill'\nexport default function App() { void backend.listRecipes(); return <div /> }\n",
      },
      {
        listRecipes: {
          name: "List recipes",
          type: "inline",
          inlineScript: {
            language: "bun",
            content: "export async function main() { return [] }\n",
          },
        },
      }
    );

    try {
      expect(helpers.listFrontendFiles()).toContain("/wmill.d.ts");
      expect(helpers.getFrontendFile("/wmill.d.ts")).toContain("listRecipes");

      const lintResult = helpers.setFrontendFile(
        "/index.tsx",
        "import { backend } from 'wmill'\nexport default function App() { void backend.deleteRecipe({ id: 1 }); return <div /> }\n"
      );

      expect(lintResult.errorCount).toBeGreaterThan(0);
      expect(lintResult.errors.frontend["/index.tsx"]?.join("\n")).toContain(
        "Property 'deleteRecipe' does not exist"
      );
    } finally {
      await cleanup();
    }
  });
});
