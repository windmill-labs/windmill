import { beforeAll, describe, expect, test } from "bun:test";
import {
  cleanupWorkspace,
  loadCliArtifactEvalCases,
  runCliArtifactEvalCase,
  shouldKeepWorkspace
} from "./artifact-eval";

const evalCases = await loadCliArtifactEvalCases();

describe("Windmill CLI Artifact Evals", () => {
  beforeAll(() => {
    if (!process.env.ANTHROPIC_API_KEY) {
      throw new Error("ANTHROPIC_API_KEY environment variable is required");
    }
  });

  for (const evalCase of evalCases) {
    test(
      evalCase.id,
      async () => {
        const result = await runCliArtifactEvalCase(evalCase);

        try {
          console.log(
            JSON.stringify(
              {
                caseId: evalCase.id,
                workspaceDir: result.workspaceDir,
                passed: result.passed,
                checks: result.checks,
                skillsInvoked: result.run.skillsInvoked,
                toolsUsed: result.run.toolsUsed.map((tool) => tool.tool),
                files: result.expectedFiles.map((file) => ({
                  path: file.path,
                  exists: file.exists
                }))
              },
              null,
              2
            )
          );

          expect(result.passed).toBe(true);
        } finally {
          if (!shouldKeepWorkspace()) {
            await cleanupWorkspace(result.workspaceDir);
          }
        }
      },
      { timeout: 180000 }
    );
  }
});
