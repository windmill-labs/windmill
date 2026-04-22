import { describe, expect, it } from "bun:test";
import { buildAppArtifacts } from "./appArtifacts";

describe("buildAppArtifacts", () => {
  it("emits lint diagnostics as an artifact alongside app files", () => {
    const artifacts = buildAppArtifacts({
      frontend: {
        "/index.tsx":
          "import { backend } from 'wmill'\nexport default function App() { void backend.deleteRecipe({ id: 1 }); return <div /> }\n",
      },
      backend: {},
      datatables: [],
    });

    const lintArtifact = artifacts.find((artifact) => artifact.path === "lint.json");
    expect(lintArtifact).toBeDefined();
    expect(lintArtifact?.content).toContain('"errorCount": 1');
    expect(lintArtifact?.content).toContain("deleteRecipe");
  });
});
