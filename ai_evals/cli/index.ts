#!/usr/bin/env bun

import {
  cleanupWorkspace,
  loadCliArtifactEvalCases,
  runCliArtifactEvalCase
} from "../adapters/cli/artifact-eval";
import {
  loadCliVariantById,
  loadCliVariants,
  snapshotCliVariant,
  type CliVariant
} from "../adapters/cli/variants";

type CommandName =
  | "run"
  | "list-cases"
  | "list-variants"
  | "snapshot-variant"
  | "compare"
  | "history";
type SurfaceName = "cli";

interface ParsedArgs {
  command: CommandName;
  surface?: string;
  caseIds: string[];
  variantIds: string[];
  description?: string;
  json: boolean;
  keepWorkspace: boolean;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));

  switch (args.command) {
    case "list-cases":
      await handleListCases(args);
      return;
    case "list-variants":
      await handleListVariants(args);
      return;
    case "snapshot-variant":
      await handleSnapshotVariant(args);
      return;
    case "run":
      await handleRun(args);
      return;
    case "compare":
      await handleCompare(args);
      return;
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

async function handleListVariants(args: ParsedArgs) {
  const surface = requireSurface(args.surface);

  switch (surface) {
    case "cli": {
      const variants = await loadCliVariants();
      const payload = {
        surface,
        variants: variants.map((entry) => ({
          id: entry.id,
          description: entry.description ?? null
        }))
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
        return;
      }

      process.stdout.write(`Surface: ${surface}\n`);
      for (const entry of payload.variants) {
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
  const caseId = requireSingleCaseId(args.caseIds);

  switch (surface) {
    case "cli": {
      const cases = await loadCliArtifactEvalCases();
      const evalCase = cases.find((entry) => entry.id === caseId);
      if (!evalCase) {
        throw new Error(`Unknown CLI case: ${caseId}`);
      }
      const variant = await loadCliVariantById(requireSingleVariantId(args.variantIds, "baseline"));

      const result = await runCliArtifactEvalCase(evalCase, { variant });
      const payload = {
        command: "run",
        surface,
        variant: variant.id,
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

async function handleSnapshotVariant(args: ParsedArgs) {
  const surface = requireSurface(args.surface);
  const variantId = requireSingleVariantId(args.variantIds, "");

  switch (surface) {
    case "cli": {
      const result = await snapshotCliVariant({
        variantId,
        description: args.description
      });

      const payload = {
        command: "snapshot-variant",
        surface,
        variant: result.variantId,
        description: result.description,
        manifestPath: result.manifestPath,
        snapshotDir: result.snapshotDir,
        usedOverrides: result.usedOverrides
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printSnapshotSummary(payload);
      }

      return;
    }
    default:
      assertNever(surface);
  }
}

async function handleCompare(args: ParsedArgs) {
  const surface = requireSurface(args.surface);

  switch (surface) {
    case "cli": {
      const allCases = await loadCliArtifactEvalCases();
      const selectedCases =
        args.caseIds.length === 0
          ? allCases
          : args.caseIds.map((caseId) => {
              const evalCase = allCases.find((entry) => entry.id === caseId);
              if (!evalCase) {
                throw new Error(`Unknown CLI case: ${caseId}`);
              }
              return evalCase;
            });

      if (args.variantIds.length < 2) {
        throw new Error("compare requires at least two --variant values");
      }

      const variants = await Promise.all(
        args.variantIds.map((variantId) => loadCliVariantById(variantId))
      );
      const labeledVariants = labelVariantSelections(variants);
      const variantResults = [];

      for (const labeledVariant of labeledVariants) {
        const caseResults = [];

        for (const evalCase of selectedCases) {
          const result = await runCliArtifactEvalCase(evalCase, {
            variant: labeledVariant.variant
          });

          try {
            caseResults.push({
              caseId: evalCase.id,
              passed: result.passed,
              skillsInvoked: result.run.skillsInvoked,
              toolsUsed: result.run.toolsUsed.map((tool) => tool.tool),
              checks: result.checks
            });
          } finally {
            await cleanupWorkspace(result.workspaceDir);
          }
        }

        variantResults.push({
          label: labeledVariant.label,
          variant: labeledVariant.variant.id,
          totalCases: caseResults.length,
          passedCases: caseResults.filter((entry) => entry.passed).length,
          caseResults
        });
      }

      const payload = {
        command: "compare",
        surface,
        caseIds: selectedCases.map((entry) => entry.id),
        variants: variantResults
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printCompareSummary(payload);
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
    caseIds: [],
    variantIds: [],
    description: undefined,
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
      parsed.caseIds.push(rest[index + 1]);
      index += 1;
      continue;
    }

    if (arg === "--variant") {
      parsed.variantIds.push(rest[index + 1]);
      index += 1;
      continue;
    }

    if (arg === "--description") {
      parsed.description = rest[index + 1];
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
    value === "list-variants" ||
    value === "snapshot-variant" ||
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

function requireSingleCaseId(caseIds: string[]): string {
  if (caseIds.length === 0) {
    throw new Error("Missing required --case argument");
  }
  if (caseIds.length > 1) {
    throw new Error("run accepts only one --case value");
  }
  return caseIds[0];
}

function requireSingleVariantId(variantIds: string[], fallback: string): string {
  if (variantIds.length === 0) {
    if (!fallback) {
      throw new Error("Missing required --variant argument");
    }
    return fallback;
  }
  if (variantIds.length > 1) {
    throw new Error("this command accepts only one --variant value");
  }
  return variantIds[0];
}

function printRunSummary(payload: {
  surface: SurfaceName;
  variant: string;
  caseId: string;
  passed: boolean;
  workspaceKept: boolean;
  workspaceDir: string | null;
  checks: Array<{ name: string; passed: boolean; required?: boolean }>;
  skillsInvoked: string[];
  toolsUsed: string[];
  expectedFiles: Array<{ path: string; exists: boolean }>;
}) {
  process.stdout.write(`Surface: ${payload.surface}\n`);
  process.stdout.write(`Variant: ${payload.variant}\n`);
  process.stdout.write(`Case: ${payload.caseId}\n`);
  process.stdout.write(`Passed: ${payload.passed ? "yes" : "no"}\n`);
  process.stdout.write(`Skills: ${payload.skillsInvoked.join(", ") || "(none)"}\n`);
  process.stdout.write(`Tools: ${payload.toolsUsed.join(", ") || "(none)"}\n`);

  if (payload.workspaceKept && payload.workspaceDir) {
    process.stdout.write(`Workspace: ${payload.workspaceDir}\n`);
  }

  process.stdout.write("Checks:\n");
  for (const check of payload.checks) {
    const marker = check.required === false ? "~" : check.passed ? "x" : " ";
    process.stdout.write(`- [${marker}] ${check.name}\n`);
  }

  process.stdout.write("Files:\n");
  for (const file of payload.expectedFiles) {
    process.stdout.write(`- ${file.path}: ${file.exists ? "present" : "missing"}\n`);
  }
}

function printCompareSummary(payload: {
  surface: SurfaceName;
  caseIds: string[];
  variants: Array<{
    label: string;
    variant: string;
    totalCases: number;
    passedCases: number;
    caseResults: Array<{ caseId: string; passed: boolean }>;
  }>;
}) {
  process.stdout.write(`Surface: ${payload.surface}\n`);
  process.stdout.write(`Cases: ${payload.caseIds.join(", ")}\n`);
  process.stdout.write("Variants:\n");

  for (const variant of payload.variants) {
    process.stdout.write(
      `- ${variant.label}: ${variant.passedCases}/${variant.totalCases} cases passed\n`
    );
    for (const caseResult of variant.caseResults) {
      process.stdout.write(
        `  ${caseResult.caseId}: ${caseResult.passed ? "pass" : "fail"}\n`
      );
    }
  }
}

function printSnapshotSummary(payload: {
  surface: SurfaceName;
  variant: string;
  description: string;
  manifestPath: string;
  snapshotDir: string;
  usedOverrides: {
    skillsSourcePath?: string;
    agentsSourcePath?: string;
    claudeSourcePath?: string;
  };
}) {
  process.stdout.write(`Surface: ${payload.surface}\n`);
  process.stdout.write(`Variant: ${payload.variant}\n`);
  process.stdout.write(`Description: ${payload.description}\n`);
  process.stdout.write(`Manifest: ${payload.manifestPath}\n`);
  process.stdout.write(`Snapshot: ${payload.snapshotDir}\n`);
  process.stdout.write("Overrides:\n");
  process.stdout.write(
    `- skills: ${payload.usedOverrides.skillsSourcePath ?? "(generated default)"}\n`
  );
  process.stdout.write(
    `- agents: ${payload.usedOverrides.agentsSourcePath ?? "(generated default)"}\n`
  );
  process.stdout.write(
    `- claude: ${payload.usedOverrides.claudeSourcePath ?? "(generated default)"}\n`
  );
}

function labelVariantSelections(
  variants: CliVariant[]
): Array<{ label: string; variant: CliVariant }> {
  const seenCounts = new Map<string, number>();

  return variants.map((variant) => {
    const count = (seenCounts.get(variant.id) ?? 0) + 1;
    seenCounts.set(variant.id, count);

    return {
      label: count === 1 ? variant.id : `${variant.id}#${count}`,
      variant
    };
  });
}

function printHelp() {
  process.stdout.write(
    [
      "Usage:",
      "  cd ai_evals && bun run cli -- list-cases --surface cli [--json]",
      "  cd ai_evals && bun run cli -- list-variants --surface cli [--json]",
      "  cd ai_evals && bun run cli -- snapshot-variant --surface cli --variant <id> [--description <text>] [--json]",
      "  cd ai_evals && bun run cli -- run --surface cli --case <id> [--variant <id>] [--json] [--keep-workspace]",
      "  cd ai_evals && bun run cli -- compare --surface cli [--case <id> ...] [--variant <id> ...] [--json]",
      "  cd ai_evals && bun run cli -- history",
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
