import { readJsonFile } from "../core/files";
import type { FrontendEvalModelConfig } from "../core/models";
import { validateScriptState } from "../core/validators";
import type { BenchmarkArtifactFile, ModeRunner } from "../core/types";
import { runScriptEval } from "../adapters/frontend/core/script/scriptEvalRunner";
import type { ScriptEvalState } from "../adapters/frontend/core/script/fileHelpers";
import { DEFAULT_FRONTEND_EVAL_MODEL, getFrontendApiKey } from "./frontendCommon";

export function createScriptModeRunner(
  modelConfig: FrontendEvalModelConfig = DEFAULT_FRONTEND_EVAL_MODEL
): ModeRunner<ScriptEvalState, ScriptEvalState, ScriptEvalState> {
  return {
    mode: "script",
    concurrency: 5,
    judgeThreshold: 80,
    async loadInitial(path) {
      return path ? await readJsonFile<ScriptEvalState>(path) : undefined;
    },
    async loadExpected(path) {
      return path ? await readJsonFile<ScriptEvalState>(path) : undefined;
    },
    async run(prompt, initial, context) {
      if (!initial) {
        throw new Error("Script evals require an initial script fixture");
      }

      const result = await runScriptEval(prompt, getFrontendApiKey(modelConfig.provider), {
        initialScript: initial,
        provider: modelConfig.provider,
        model: modelConfig.model,
        runContext: context,
      });

      return {
        success: result.success,
        actual: result.script,
        error: result.error,
        assistantMessageCount: result.assistantMessageCount,
        toolCallCount: result.toolCallCount,
        toolsUsed: result.toolsUsed,
        skillsInvoked: [],
        tokenUsage: result.tokenUsage,
      };
    },
    validate({ actual, initial, expected }) {
      return validateScriptState({ actual, initial, expected });
    },
    buildArtifacts(actual): BenchmarkArtifactFile[] {
      return [
        {
          path: "script.json",
          content: JSON.stringify(actual, null, 2) + "\n",
        },
        {
          path: actual.path,
          content: actual.code,
        },
      ];
    },
  };
}
