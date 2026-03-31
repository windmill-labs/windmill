import { beforeAll, describe, expect, test } from "bun:test";
import {
  cleanupWorkspace,
  loadCliArtifactEvalCases,
  runCliArtifactEvalCase,
  shouldKeepWorkspace
} from "../../../ai_evals/adapters/cli/artifact-eval";

const smokeCaseIds = new Set(["bun-hello-script"]);
const evalCases = (await loadCliArtifactEvalCases()).filter((evalCase) =>
  smokeCaseIds.has(evalCase.id)
);

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
