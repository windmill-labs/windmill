import { readJsonFile } from "../core/files";
import { validateScriptState } from "../core/validators";
import type { ModeRunner } from "../core/types";
import { runScriptEval } from "../adapters/frontend/core/script/scriptEvalRunner";
import type { ScriptEvalState } from "../adapters/frontend/core/script/fileHelpers";
import { FRONTEND_MODEL, FRONTEND_PROVIDER, getFrontendApiKey } from "./frontendCommon";

export function createScriptModeRunner(): ModeRunner<ScriptEvalState, ScriptEvalState, ScriptEvalState> {
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

      const result = await runScriptEval(prompt, getFrontendApiKey(), {
        initialScript: initial,
        provider: FRONTEND_PROVIDER,
        model: FRONTEND_MODEL,
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
      };
    },
    validate({ actual, initial, expected }) {
      return validateScriptState({ actual, initial, expected });
    },
  };
}
