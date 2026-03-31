import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const MODULE_DIR = path.dirname(fileURLToPath(import.meta.url));

export const DEFAULT_HISTORY_DIR = MODULE_DIR;
const SUMMARY_FILENAME = "summary.jsonl";
const RUNS_DIRNAME = "runs";
const ROLLUPS_DIRNAME = "rollups";

export async function appendOfficialRun(input, options = {}) {
  const absoluteHistoryDir = path.resolve(options.historyDir ?? DEFAULT_HISTORY_DIR);
  const normalizedRun = normalizeRun(input);
  await ensureHistoryLayout(absoluteHistoryDir);

  const runFilename = `${normalizedRun.run_id}.json`;
  const runRelativePath = path.posix.join(RUNS_DIRNAME, runFilename);
  const runFilePath = path.join(absoluteHistoryDir, runRelativePath);

  await writeJsonFile(runFilePath, normalizedRun);

  const summaryPath = path.join(absoluteHistoryDir, SUMMARY_FILENAME);
  const summaries = await loadSummaryEntries(summaryPath);
  const summaryEntry = buildSummaryEntry(normalizedRun, runRelativePath);
  const nextSummaries = upsertSummaryEntry(summaries, summaryEntry);

  await writeSummaryEntries(summaryPath, nextSummaries);
  await writeRollups(absoluteHistoryDir, nextSummaries);

  return {
    status: "ok",
    runId: normalizedRun.run_id,
    runPath: runRelativePath,
    summaryEntries: nextSummaries.length
  };
}

export async function loadLatestHistoryRollup(historyDir = DEFAULT_HISTORY_DIR) {
  const rollupPath = path.join(path.resolve(historyDir), ROLLUPS_DIRNAME, "latest.json");
  return await loadJsonFile(rollupPath);
}

export async function loadHistoryRollup(
  name,
  historyDir = DEFAULT_HISTORY_DIR
) {
  const rollupPath = path.join(path.resolve(historyDir), ROLLUPS_DIRNAME, name);
  return await loadJsonFile(rollupPath);
}

export async function loadSummaryHistory(historyDir = DEFAULT_HISTORY_DIR) {
  const summaryPath = path.join(path.resolve(historyDir), SUMMARY_FILENAME);
  return await loadSummaryEntries(summaryPath);
}

async function loadJsonFile(filePath) {
  const raw = await readFile(filePath, "utf8");
  try {
    return JSON.parse(raw);
  } catch (error) {
    throw new Error(`Failed to parse JSON from ${filePath}: ${error.message}`);
  }
}

function normalizeRun(input) {
  assertPlainObject(input, "benchmark run");

  const timestamp = assertIsoDateTime(input.timestamp, "timestamp");
  const gitSha = assertNonEmptyString(input.git_sha, "git_sha");
  const suiteVersion = assertNonEmptyString(input.suite_version, "suite_version");
  const scoringVersion = assertNonEmptyString(
    input.scoring_version,
    "scoring_version"
  );
  const surface = assertNonEmptyString(input.surface, "surface");
  const variantName = assertNonEmptyString(input.variant_name, "variant_name");
  const provider = assertNonEmptyString(input.provider, "provider");
  const model = assertNonEmptyString(input.model, "model");
  const judgeModel =
    input.judge_model === null || input.judge_model === undefined
      ? null
      : assertNonEmptyString(input.judge_model, "judge_model");
  const runsPerCase = assertPositiveInteger(input.runs_per_case, "runs_per_case");
  const caseCount = assertPositiveInteger(input.case_count, "case_count");
  const metrics = normalizeMetrics(input.metrics, runsPerCase);
  const cases = normalizeCases(input.cases);

  if (cases.length !== caseCount) {
    throw new Error(
      `case_count (${caseCount}) does not match cases.length (${cases.length})`
    );
  }

  const runId =
    input.run_id && typeof input.run_id === "string" && input.run_id.trim()
      ? input.run_id.trim()
      : buildRunId({
          timestamp,
          surface,
          variantName,
          provider,
          model,
          gitSha
        });

  return {
    ...input,
    run_id: runId,
    timestamp,
    git_sha: gitSha,
    suite_version: suiteVersion,
    scoring_version: scoringVersion,
    surface,
    variant_name: variantName,
    provider,
    model,
    judge_model: judgeModel,
    runs_per_case: runsPerCase,
    case_count: caseCount,
    metrics,
    cases
  };
}

function normalizeMetrics(input, runsPerCase) {
  assertPlainObject(input, "metrics");

  const quality = normalizeMetricGroup(
    input.quality,
    "metrics.quality",
    [
      "pass_rate",
      "deterministic_pass_rate",
      "judge_score_mean",
      "judge_score_median",
      "judge_score_p10",
      "quality_score"
    ],
    new Set(["pass_rate", "deterministic_pass_rate"])
  );
  const reliability = normalizeMetricGroup(
    input.reliability,
    "metrics.reliability",
    ["runs_per_case", "flake_rate", "path_consistency"],
    new Set(["flake_rate", "path_consistency"])
  );
  const efficiency = normalizeMetricGroup(
    input.efficiency,
    "metrics.efficiency",
    [
      "latency_ms_mean",
      "latency_ms_median",
      "tokens_total_mean",
      "tool_calls_mean",
      "iterations_mean",
      "estimated_cost_mean",
      "cost_per_success",
      "latency_per_success",
      "efficiency_score",
      "value_score"
    ]
  );

  if (reliability.runs_per_case !== runsPerCase) {
    throw new Error(
      `metrics.reliability.runs_per_case (${reliability.runs_per_case}) does not match runs_per_case (${runsPerCase})`
    );
  }

  if (quality.category_pass_rate !== undefined) {
    assertPlainObject(quality.category_pass_rate, "metrics.quality.category_pass_rate");
    for (const [category, value] of Object.entries(quality.category_pass_rate)) {
      assertRatio(value, `metrics.quality.category_pass_rate.${category}`);
    }
  }

  return { quality, reliability, efficiency };
}

function normalizeMetricGroup(input, groupName, requiredFields, ratioFields = new Set()) {
  assertPlainObject(input, groupName);
  const normalized = { ...input };

  for (const field of requiredFields) {
    if (!(field in normalized)) {
      throw new Error(`Missing required ${groupName}.${field}`);
    }
    if (field === "runs_per_case") {
      normalized[field] = assertPositiveInteger(
        normalized[field],
        `${groupName}.${field}`
      );
      continue;
    }
    normalized[field] = assertFiniteNumber(normalized[field], `${groupName}.${field}`);
    if (ratioFields.has(field)) {
      assertRatio(normalized[field], `${groupName}.${field}`);
    }
  }

  for (const [field, value] of Object.entries(normalized)) {
    if (value === null || value === undefined) {
      continue;
    }
    if (typeof value === "number") {
      normalized[field] = assertFiniteNumber(value, `${groupName}.${field}`);
      if (ratioFields.has(field)) {
        assertRatio(normalized[field], `${groupName}.${field}`);
      }
    }
  }

  return normalized;
}

function normalizeCases(input) {
  if (!Array.isArray(input) || input.length === 0) {
    throw new Error("cases must be a non-empty array");
  }

  return input.map((entry, index) => {
    assertPlainObject(entry, `cases[${index}]`);

    return {
      ...entry,
      id: assertNonEmptyString(entry.id, `cases[${index}].id`),
      pass_rate: assertRatio(entry.pass_rate, `cases[${index}].pass_rate`)
    };
  });
}

function buildRunId({ timestamp, surface, variantName, provider, model, gitSha }) {
  const timestampSlug = timestamp.replaceAll(":", "-").replaceAll(".", "-");
  const shortSha = gitSha.slice(0, 12);

  return [
    slugify(timestampSlug),
    slugify(surface),
    slugify(variantName),
    slugify(provider),
    slugify(model),
    shortSha
  ]
    .filter(Boolean)
    .join("__");
}

function buildSummaryEntry(run, runRelativePath) {
  return {
    run_id: run.run_id,
    timestamp: run.timestamp,
    git_sha: run.git_sha,
    suite_version: run.suite_version,
    scoring_version: run.scoring_version,
    surface: run.surface,
    variant_name: run.variant_name,
    provider: run.provider,
    model: run.model,
    judge_model: run.judge_model,
    runs_per_case: run.runs_per_case,
    case_count: run.case_count,
    run_path: runRelativePath,
    metrics: run.metrics
  };
}

function upsertSummaryEntry(entries, nextEntry) {
  const remainingEntries = entries.filter((entry) => entry.run_id !== nextEntry.run_id);
  const nextEntries = [...remainingEntries, nextEntry];
  nextEntries.sort((left, right) => left.timestamp.localeCompare(right.timestamp));
  return nextEntries;
}

async function loadSummaryEntries(summaryPath) {
  try {
    const raw = await readFile(summaryPath, "utf8");
    return raw
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line, index) => {
        try {
          return JSON.parse(line);
        } catch (error) {
          throw new Error(
            `Failed to parse ${SUMMARY_FILENAME} line ${index + 1}: ${error.message}`
          );
        }
      });
  } catch (error) {
    if (error.code === "ENOENT") {
      return [];
    }
    throw error;
  }
}

async function writeSummaryEntries(summaryPath, entries) {
  const content =
    entries.map((entry) => JSON.stringify(entry)).join("\n") +
    (entries.length > 0 ? "\n" : "");
  await writeFile(summaryPath, content, "utf8");
}

async function writeRollups(historyDir, summaries) {
  const rollupsDir = path.join(historyDir, ROLLUPS_DIRNAME);
  const generatedAt = new Date().toISOString();
  const latestFirst = [...summaries].sort((left, right) =>
    right.timestamp.localeCompare(left.timestamp)
  );
  const latestBySurface = {};

  for (const summary of latestFirst) {
    if (!(summary.surface in latestBySurface)) {
      latestBySurface[summary.surface] = summary;
    }
  }

  const latestRollup = {
    generatedAt,
    latestRun: latestFirst[0] ?? null,
    latestBySurface
  };
  const bySurfaceRollup = {
    generatedAt,
    groupKey: "surface",
    groups: groupSummaries(summaries, (summary) => summary.surface)
  };
  const byVariantRollup = {
    generatedAt,
    groupKey: "surface:variant_name",
    groups: groupSummaries(
      summaries,
      (summary) => `${summary.surface}:${summary.variant_name}`
    )
  };
  const byModelRollup = {
    generatedAt,
    groupKey: "provider:model",
    groups: groupSummaries(
      summaries,
      (summary) => `${summary.provider}:${summary.model}`
    )
  };

  await Promise.all([
    writeJsonFile(path.join(rollupsDir, "latest.json"), latestRollup),
    writeJsonFile(path.join(rollupsDir, "by_surface.json"), bySurfaceRollup),
    writeJsonFile(path.join(rollupsDir, "by_variant.json"), byVariantRollup),
    writeJsonFile(path.join(rollupsDir, "by_model.json"), byModelRollup)
  ]);
}

function groupSummaries(summaries, getGroupKey) {
  const groups = {};

  for (const summary of summaries) {
    const key = getGroupKey(summary);
    groups[key] ??= [];
    groups[key].push(summary);
  }

  for (const groupEntries of Object.values(groups)) {
    groupEntries.sort((left, right) => left.timestamp.localeCompare(right.timestamp));
  }

  return groups;
}

async function ensureHistoryLayout(historyDir) {
  await Promise.all([
    mkdir(path.join(historyDir, RUNS_DIRNAME), { recursive: true }),
    mkdir(path.join(historyDir, ROLLUPS_DIRNAME), { recursive: true })
  ]);
}

async function writeJsonFile(filePath, value) {
  await writeFile(filePath, JSON.stringify(value, null, 2) + "\n", "utf8");
}

function assertPlainObject(value, fieldName) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${fieldName} must be an object`);
  }
}

function assertNonEmptyString(value, fieldName) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${fieldName} must be a non-empty string`);
  }
  return value.trim();
}

function assertPositiveInteger(value, fieldName) {
  if (!Number.isInteger(value) || value < 1) {
    throw new Error(`${fieldName} must be a positive integer`);
  }
  return value;
}

function assertFiniteNumber(value, fieldName) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`${fieldName} must be a finite number`);
  }
  return value;
}

function assertRatio(value, fieldName) {
  const normalized = assertFiniteNumber(value, fieldName);
  if (normalized < 0 || normalized > 1) {
    throw new Error(`${fieldName} must be between 0 and 1`);
  }
  return normalized;
}

function assertIsoDateTime(value, fieldName) {
  const normalized = assertNonEmptyString(value, fieldName);
  if (Number.isNaN(Date.parse(normalized))) {
    throw new Error(`${fieldName} must be a valid ISO date-time string`);
  }
  return normalized;
}

function slugify(value) {
  return value
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/g, "-")
    .replaceAll(/^-+|-+$/g, "");
}
