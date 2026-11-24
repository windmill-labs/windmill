import { describe, expect, it } from "bun:test";
import { formatPreviewFailureDetails } from "./previewFailureDetails";

describe("formatPreviewFailureDetails", () => {
  it("includes runtime logs and result excerpts for failed preview jobs", () => {
    const details = formatPreviewFailureDetails({
      workspaceId: "ai-evals-crud-a1",
      jobId: "job-123",
      logs: "\nTypeError: Cannot read properties of undefined\n    at listRecipes\n",
      result: { error: "frontend called missing backend runnable" },
    });

    expect(details).toContain("workspace=ai-evals-crud-a1");
    expect(details).toContain("job=job-123");
    expect(details).toContain("TypeError: Cannot read properties of undefined");
    expect(details).toContain("frontend called missing backend runnable");
  });

  it("bounds noisy logs so check summaries stay readable", () => {
    const details = formatPreviewFailureDetails({
      workspaceId: "workspace",
      jobId: "job",
      logs: "x".repeat(100),
      maxCharacters: 20,
    });

    expect(details).toContain(`logs:\n…${"x".repeat(19)}`);
    expect(details.length).toBeLessThan(80);
  });

  it("keeps the error at the end of a log after long install output", () => {
    const installOutput = Array.from(
      { length: 150 },
      (_, i) => `+ package-${i}@1.0.${i} installed from the registry`,
    ).join("\n");
    const details = formatPreviewFailureDetails({
      workspaceId: "workspace",
      jobId: "job",
      logs: `${installOutput}\nerror: Cannot find module 'missing'\n    at main (/tmp/main.ts:3:9)\n`,
      result: {
        error: {
          name: "Error",
          stack: "x".repeat(2000),
          message: "Cannot find module",
        },
      },
    });

    expect(details).toContain(
      "error: Cannot find module 'missing'\n    at main (/tmp/main.ts:3:9)",
    );
    expect(details).not.toContain("package-0@");
    expect(details).toContain("result:\nError: Cannot find module\nxxx");
  });
});
