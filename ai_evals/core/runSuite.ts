import { judgeOutput, DEFAULT_JUDGE_MODEL } from "./judge";
import type {
  BenchmarkAttemptResult,
  BenchmarkCaseResult,
  BenchmarkCheck,
  EvalCase,
  FrontendBenchmarkProgressEvent,
  ModeRunner,
} from "./types";

export async function runSuite<TInitial, TExpected, TActual>(input: {
  modeRunner: ModeRunner<TInitial, TExpected, TActual>;
  cases: EvalCase[];
  runs: number;
  runModel: string | null;
  judgeModel?: string | null;
  concurrency?: number;
  verbose?: boolean;
  onProgress?: (event: FrontendBenchmarkProgressEvent) => void;
}): Promise<BenchmarkCaseResult[]> {
  const judgeModel = input.judgeModel ?? DEFAULT_JUDGE_MODEL;
  const concurrency = Math.max(1, input.concurrency ?? input.modeRunner.concurrency);
  const results = new Array<BenchmarkCaseResult>(input.cases.length);
  let cursor = 0;

  if (input.modeRunner.mode !== "cli") {
    input.onProgress?.({
      type: "run-start",
      surface: input.modeRunner.mode,
      totalCases: input.cases.length,
      runs: input.runs,
      concurrency,
    });
  }

  async function worker(): Promise<void> {
    while (true) {
      const caseIndex = cursor++;
      if (caseIndex >= input.cases.length) {
        return;
      }
      const evalCase = input.cases[caseIndex];
      results[caseIndex] = {
        id: evalCase.id,
        prompt: evalCase.prompt,
        initialPath: evalCase.initialPath,
        expectedPath: evalCase.expectedPath,
        attempts: await runCaseAttempts({
          caseIndex,
          evalCase,
          runs: input.runs,
          judgeModel,
          judgeThreshold: input.modeRunner.judgeThreshold ?? 80,
          modeRunner: input.modeRunner,
          totalCases: input.cases.length,
          verbose: input.verbose ?? false,
          onProgress: input.onProgress,
        }),
      };
    }
  }

  await Promise.all(
    Array.from({ length: Math.min(concurrency, input.cases.length) }, () => worker())
  );

  return results;
}

async function runCaseAttempts<TInitial, TExpected, TActual>(input: {
  caseIndex: number;
  evalCase: EvalCase;
  runs: number;
  judgeModel: string;
  judgeThreshold: number;
  modeRunner: ModeRunner<TInitial, TExpected, TActual>;
  totalCases: number;
  verbose: boolean;
  onProgress?: (event: FrontendBenchmarkProgressEvent) => void;
}): Promise<BenchmarkAttemptResult[]> {
  const attempts: BenchmarkAttemptResult[] = [];
  const surface = input.modeRunner.mode === "cli" ? null : input.modeRunner.mode;

  for (let attempt = 1; attempt <= input.runs; attempt += 1) {
    if (surface) {
      input.onProgress?.({
        type: "attempt-start",
        surface,
        caseId: input.evalCase.id,
        caseNumber: input.caseIndex + 1,
        totalCases: input.totalCases,
        attempt,
        runs: input.runs,
      });
    }

    const startedAt = Date.now();

    try {
      const initial = await input.modeRunner.loadInitial(input.evalCase.initialPath);
      const expected = await input.modeRunner.loadExpected(input.evalCase.expectedPath);
      const run = await input.modeRunner.run(input.evalCase.prompt, initial, {
        caseId: input.evalCase.id,
        caseNumber: input.caseIndex + 1,
        totalCases: input.totalCases,
        attempt,
        runs: input.runs,
        verbose: input.verbose,
        onAssistantMessageStart: input.verbose && surface
          ? () =>
              input.onProgress?.({
                type: "assistant-message-start",
                surface,
                caseId: input.evalCase.id,
                caseNumber: input.caseIndex + 1,
                totalCases: input.totalCases,
                attempt,
                runs: input.runs,
              })
          : undefined,
        onAssistantChunk: input.verbose && surface
          ? (chunk: string) =>
              input.onProgress?.({
                type: "assistant-chunk",
                surface,
                caseId: input.evalCase.id,
                caseNumber: input.caseIndex + 1,
                totalCases: input.totalCases,
                attempt,
                runs: input.runs,
                chunk,
              })
          : undefined,
        onAssistantMessageEnd: input.verbose && surface
          ? () =>
              input.onProgress?.({
                type: "assistant-message-end",
                surface,
                caseId: input.evalCase.id,
                caseNumber: input.caseIndex + 1,
                totalCases: input.totalCases,
                attempt,
                runs: input.runs,
              })
          : undefined,
      });
      const checks: BenchmarkCheck[] = [
        buildCheck("run succeeded", run.success, run.error),
        ...input.modeRunner.validate({
          evalCase: input.evalCase,
          prompt: input.evalCase.prompt,
          initial,
          expected,
          actual: run.actual,
          run,
        }),
      ];
      const artifactFiles = input.modeRunner.buildArtifacts?.(run.actual) ?? [];

      if (run.success && input.modeRunner.backendValidate) {
        try {
          const backendValidation = await input.modeRunner.backendValidate({
            evalCase: input.evalCase,
            prompt: input.evalCase.prompt,
            initial,
            expected,
            actual: run.actual,
            run,
            context: {
              caseId: input.evalCase.id,
              caseNumber: input.caseIndex + 1,
              totalCases: input.totalCases,
              attempt,
              runs: input.runs,
              verbose: input.verbose,
              onAssistantMessageStart: undefined,
              onAssistantChunk: undefined,
              onAssistantMessageEnd: undefined,
            },
          });

          if (backendValidation) {
            checks.push(...backendValidation.checks);
            artifactFiles.push(...(backendValidation.artifactFiles ?? []));
          }
        } catch (error) {
          checks.push(
            buildCheck(
              "backend validation succeeded",
              false,
              error instanceof Error ? error.message : String(error)
            )
          );
        }
      }

      let judgeScore: number | null = null;
      let judgeSummary: string | null = null;

      if (run.success) {
        const judge = await judgeOutput({
          mode: input.modeRunner.mode,
          prompt: input.evalCase.prompt,
          checklist: input.evalCase.judgeChecklist,
          initial,
          expected: input.modeRunner.mode === "cli" ? undefined : expected,
          actual: run.actual,
          model: input.judgeModel,
        });

        judgeScore = judge.success ? judge.score : null;
        judgeSummary = judge.summary;
        checks.push(buildCheck("judge succeeded", judge.success, judge.error));
        checks.push(
          buildCheck(
            `judge score >= ${input.judgeThreshold}`,
            (judgeScore ?? 0) >= input.judgeThreshold,
            judge.success ? `score=${judgeScore}` : judge.error
          )
        );
      }

      const attemptResult: BenchmarkAttemptResult = {
        attempt,
        passed: checks.every((check) => check.passed),
        durationMs: Date.now() - startedAt,
        assistantMessageCount: run.assistantMessageCount,
        toolCallCount: run.toolCallCount,
        toolsUsed: uniqueStrings(run.toolsUsed),
        skillsInvoked: uniqueStrings(run.skillsInvoked),
        checks,
        judgeScore,
        judgeSummary,
        error: run.error ?? null,
        tokenUsage: run.tokenUsage ?? null,
        artifactsPath: null,
        artifactFiles,
      };

      if (surface) {
        input.onProgress?.({
          type: "attempt-finish",
          surface,
          caseId: input.evalCase.id,
          caseNumber: input.caseIndex + 1,
          totalCases: input.totalCases,
          attempt,
          runs: input.runs,
          passed: attemptResult.passed,
          durationMs: attemptResult.durationMs,
          judgeScore: attemptResult.judgeScore,
          error: attemptResult.error,
        });
      }

      attempts.push(attemptResult);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const failedAttempt: BenchmarkAttemptResult = {
        attempt,
        passed: false,
        durationMs: Date.now() - startedAt,
        assistantMessageCount: 0,
        toolCallCount: 0,
        toolsUsed: [],
        skillsInvoked: [],
        checks: [buildCheck("run crashed", false, message)],
        judgeScore: null,
        judgeSummary: null,
        error: message,
        tokenUsage: null,
      };
      if (surface) {
        input.onProgress?.({
          type: "attempt-finish",
          surface,
          caseId: input.evalCase.id,
          caseNumber: input.caseIndex + 1,
          totalCases: input.totalCases,
          attempt,
          runs: input.runs,
          passed: false,
          durationMs: failedAttempt.durationMs,
          judgeScore: null,
          error: message,
        });
      }
      attempts.push(failedAttempt);
    }
  }

  return attempts;
}

function buildCheck(name: string, passed: boolean, details?: string): BenchmarkCheck {
  return details ? { name, passed, details } : { name, passed };
}

function uniqueStrings(values: string[]): string[] {
  return [...new Set(values)];
}
