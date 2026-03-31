#!/usr/bin/env bun

import {
  cleanupWorkspace,
  loadCliArtifactEvalCases,
  runCliArtifactEvalCase
} from "../../cli/test-skills/src/artifact-eval";

type CommandName = "run" | "list-cases" | "compare" | "history";
type SurfaceName = "cli";

interface ParsedArgs {
  command: CommandName;
  surface?: string;
  caseId?: string;
  json: boolean;
  keepWorkspace: boolean;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));

  switch (args.command) {
    case "list-cases":
      await handleListCases(args);
      return;
    case "run":
      await handleRun(args);
      return;
    case "compare":
    case "history":
      throw new Error(
        `'${args.command}' is not implemented yet in the repo-level benchmark CLI`
      );
    default:
      printHelp();
      throw new Error(`Unknown command: ${String(args.command)}`);
  }
}

async function handleListCases(args: ParsedArgs) {
  const surface = requireSurface(args.surface);
  switch (surface) {
    case "cli": {
      const cases = await loadCliArtifactEvalCases();
      const payload = {
        surface,
        cases: cases.map((entry) => ({
          id: entry.id,
          description: entry.description ?? null
        }))
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
        return;
      }

      process.stdout.write(`Surface: ${surface}\n`);
      for (const entry of payload.cases) {
        process.stdout.write(
          `- ${entry.id}${entry.description ? `: ${entry.description}` : ""}\n`
        );
      }
      return;
    }
    default:
      assertNever(surface);
  }
}

async function handleRun(args: ParsedArgs) {
  const surface = requireSurface(args.surface);
  const caseId = requireCaseId(args.caseId);

  switch (surface) {
    case "cli": {
      const cases = await loadCliArtifactEvalCases();
      const evalCase = cases.find((entry) => entry.id === caseId);
      if (!evalCase) {
        throw new Error(`Unknown CLI case: ${caseId}`);
      }

      const result = await runCliArtifactEvalCase(evalCase);
      const payload = {
        command: "run",
        surface,
        caseId: evalCase.id,
        passed: result.passed,
        workspaceKept: args.keepWorkspace,
        workspaceDir: args.keepWorkspace ? result.workspaceDir : null,
        checks: result.checks,
        skillsInvoked: result.run.skillsInvoked,
        toolsUsed: result.run.toolsUsed.map((tool) => tool.tool),
        expectedFiles: result.expectedFiles.map((file) => ({
          path: file.path,
          exists: file.exists
        }))
      };

      try {
        if (args.json) {
          process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
        } else {
          printRunSummary(payload);
        }
      } finally {
        if (!args.keepWorkspace) {
          await cleanupWorkspace(result.workspaceDir);
        }
      }

      if (!result.passed) {
        process.exitCode = 1;
      }
      return;
    }
    default:
      assertNever(surface);
  }
}

function parseArgs(argv: string[]): ParsedArgs {
  const [commandArg, ...rest] = argv;

  if (!commandArg || commandArg === "--help" || commandArg === "-h") {
    printHelp();
    process.exit(0);
  }

  if (!isCommandName(commandArg)) {
    throw new Error(`Unknown command: ${commandArg}`);
  }

  const parsed: ParsedArgs = {
    command: commandArg,
    json: false,
    keepWorkspace: false
  };

  for (let index = 0; index < rest.length; index += 1) {
    const arg = rest[index];

    if (arg === "--surface") {
      parsed.surface = rest[index + 1];
      index += 1;
      continue;
    }

    if (arg === "--case") {
      parsed.caseId = rest[index + 1];
      index += 1;
      continue;
    }

    if (arg === "--json") {
      parsed.json = true;
      continue;
    }

    if (arg === "--keep-workspace") {
      parsed.keepWorkspace = true;
      continue;
    }

    if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    }

    throw new Error(`Unknown argument: ${arg}`);
  }

  return parsed;
}

function isCommandName(value: string): value is CommandName {
  return (
    value === "run" ||
    value === "list-cases" ||
    value === "compare" ||
    value === "history"
  );
}

function requireSurface(surface: string | undefined): SurfaceName {
  if (!surface) {
    throw new Error("Missing required --surface argument");
  }
  if (surface !== "cli") {
    throw new Error(`Unsupported surface for now: ${surface}`);
  }
  return surface;
}

function requireCaseId(caseId: string | undefined): string {
  if (!caseId) {
    throw new Error("Missing required --case argument");
  }
  return caseId;
}

function printRunSummary(payload: {
  surface: SurfaceName;
  caseId: string;
  passed: boolean;
  workspaceKept: boolean;
  workspaceDir: string | null;
  checks: Array<{ name: string; passed: boolean }>;
  skillsInvoked: string[];
  toolsUsed: string[];
  expectedFiles: Array<{ path: string; exists: boolean }>;
}) {
  process.stdout.write(`Surface: ${payload.surface}\n`);
  process.stdout.write(`Case: ${payload.caseId}\n`);
  process.stdout.write(`Passed: ${payload.passed ? "yes" : "no"}\n`);
  process.stdout.write(`Skills: ${payload.skillsInvoked.join(", ") || "(none)"}\n`);
  process.stdout.write(`Tools: ${payload.toolsUsed.join(", ") || "(none)"}\n`);

  if (payload.workspaceKept && payload.workspaceDir) {
    process.stdout.write(`Workspace: ${payload.workspaceDir}\n`);
  }

  process.stdout.write("Checks:\n");
  for (const check of payload.checks) {
    process.stdout.write(`- [${check.passed ? "x" : " "}] ${check.name}\n`);
  }

  process.stdout.write("Files:\n");
  for (const file of payload.expectedFiles) {
    process.stdout.write(`- ${file.path}: ${file.exists ? "present" : "missing"}\n`);
  }
}

function printHelp() {
  process.stdout.write(
    [
      "Usage:",
      "  bun ai_evals/cli/index.ts list-cases --surface cli [--json]",
      "  bun ai_evals/cli/index.ts run --surface cli --case <id> [--json] [--keep-workspace]",
      "  bun ai_evals/cli/index.ts compare",
      "  bun ai_evals/cli/index.ts history",
      "",
      "Current support:",
      "  surfaces: cli"
    ].join("\n") + "\n"
  );
}

function assertNever(value: never): never {
  throw new Error(`Unexpected value: ${String(value)}`);
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
});
