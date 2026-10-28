import { Command } from "@cliffy/command";
import { colors } from "@cliffy/ansi/colors";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { posix } from "node:path";

import * as log from "../../core/log.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { GlobalOptions, getTypeStrFromPath, parseDatatableMigrationPath } from "../../types.ts";
import {
  DBT_MODULE_SUFFIX,
  DOTTED_SUFFIXES,
  NON_DOTTED_SUFFIXES,
} from "../../utils/resource_folders.ts";
import { apiErrorMessage } from "../../utils/utils.ts";
import { yamlParseContent } from "../../utils/yaml.ts";
import {
  capText,
  folderInstructionScope,
  instructionsCovering,
  MAX_OUTPUT_TOKENS,
  parseReviewAnswer,
  PER_SIDE_CHAR_CAP,
  REVIEW_INSTRUCTIONS_RESOURCE_TYPE,
  ReviewBudget,
  reviewInstructionsSection,
  reviewSystemPrompt,
  VALUE_WITHHELD_KINDS,
  type AiReviewFinding,
  type AiReviewSeverity,
  type FolderInstruction,
} from "./aiReviewCore.gen.ts";

/** Context GitHub's diff gives a pull request, and so the lines a review comment can sit on. */
const GITHUB_DIFF_CONTEXT = 3;
/** Context the model reads around each change. */
const MODEL_DIFF_CONTEXT = 20;

type ReviewOptions = GlobalOptions & {
  base?: string;
  head?: string;
  model?: string;
  json?: boolean;
  post?: "github";
  pr?: number;
  failOn?: AiReviewSeverity;
};

type ChangedFile = { file: string; status: "added" | "deleted" | "modified" };
type ChangedItem = { kind: string; path: string; files: ChangedFile[] };

export type ReviewResult = {
  summary: string;
  findings: AiReviewFinding[];
  /** Changed items reviewed by name only, over the size budget. */
  omitted: string[];
  /** Changed items whose files could not be read as text, so the model did not review them. */
  unreadable: string[];
  /** Review instructions of the base branch that covered a changed item. */
  instructions: string[];
  /** Review instructions on the base branch whose body could not be read. */
  skippedInstructions: string[];
  model: string;
};

function git(args: string[]): string {
  return execFileSync("git", ["-c", "core.quotePath=false", ...args], {
    encoding: "utf-8",
    maxBuffer: 512 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  });
}

/** The workspace item a repository file belongs to, as git sync lays a workspace out. Any
 * other file is its own item of kind `file`. */
export function itemOf(file: string): { kind: string; path: string } {
  // The outermost container owns everything under it: a fileset's or a dbt project's
  // children are named freely, so a child called `x.flow/` or `legacy__mod/` is still
  // part of the resource or script around it.
  const containers: [string, string][] = [
    [".fileset/", "resource"],
    ["__mod/", "script"],
    [`${DBT_MODULE_SUFFIX}/`, "script"],
    ...[DOTTED_SUFFIXES, NON_DOTTED_SUFFIXES].flatMap((suffixes) =>
      Object.entries(suffixes).map(([kind, suffix]): [string, string] => [`${suffix}/`, kind])
    ),
  ];
  let outermost: { kind: string; path: string } | undefined;
  for (const [marker, kind] of containers) {
    const i = file.indexOf(marker);
    if (i > 0 && (!outermost || i < outermost.path.length)) {
      outermost = { kind, path: file.slice(0, i) };
    }
  }
  if (outermost) return outermost;
  if (parseDatatableMigrationPath(file)) return { kind: "datatable_migration", path: file };
  const sidecar = file.indexOf(".resource.file.");
  if (sidecar > 0) return { kind: "resource", path: file.slice(0, sidecar) };
  let kind: string;
  try {
    kind = getTypeStrFromPath(file);
  } catch {
    return { kind: "file", path: file };
  }
  if (kind === "folder") return { kind, path: posix.dirname(file) };
  const dir = posix.dirname(file);
  const name = posix.basename(file).split(".")[0];
  if (!/^[ufg]\//.test(file) || !name) return { kind, path: file };
  return { kind, path: dir === "." ? name : `${dir}/${name}` };
}

/** Lockfiles are left out of the review, as the deployment request review drops `lock`. */
function isLockfile(file: string): boolean {
  return posix.basename(file) === "wmill-lock.yaml" || file.endsWith(".lock");
}

/**
 * A unified diff with every line prefixed by its line number, R<n> on the pull request's
 * side and L<n> on the base's, and the numbers each side shows. `undefined` for a file git
 * reports as binary.
 */
export function numberDiff(
  diff: string,
): { text: string; left: Set<number>; right: Set<number> } | undefined {
  if (/^Binary files .* differ$/m.test(diff)) return undefined;
  const out: string[] = [];
  const left = new Set<number>();
  const right = new Set<number>();
  let l = 0;
  let r = 0;
  let inHunk = false;
  for (const line of diff.split("\n")) {
    const hunk = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (hunk) {
      l = Number(hunk[1]);
      r = Number(hunk[2]);
      inHunk = true;
      out.push(line);
      continue;
    }
    if (!inHunk || line.startsWith("\\")) continue;
    const mark = line[0];
    const content = line.slice(1);
    if (mark === "+") {
      right.add(r);
      out.push(`R${r++} +${content}`);
    } else if (mark === "-") {
      left.add(l);
      out.push(`L${l++} -${content}`);
    } else if (mark === " ") {
      right.add(r);
      out.push(`R${r++}  ${content}`);
      l++;
    }
  }
  return { text: out.join("\n"), left, right };
}

function changedItems(mergeBase: string, head: string): ChangedItem[] {
  const out = git(["diff", "--name-status", "--no-renames", "-z", mergeBase, head]);
  const fields = out.split("\0").filter((f) => f !== "");
  const items = new Map<string, ChangedItem>();
  for (let i = 0; i + 1 < fields.length; i += 2) {
    const [code, file] = [fields[i], fields[i + 1]];
    if (isLockfile(file)) continue;
    const { kind, path } = itemOf(file);
    const key = `${kind}:${path}`;
    const item = items.get(key) ?? { kind, path, files: [] };
    item.files.push({
      file,
      status: code === "A" ? "added" : code === "D" ? "deleted" : "modified",
    });
    items.set(key, item);
  }
  return [...items.values()];
}

function changeOf(item: ChangedItem): string {
  if (item.files.every((f) => f.status === "added")) return "added";
  if (item.files.every((f) => f.status === "deleted")) return "deleted";
  return "modified";
}

/** Review instructions are read from the base branch, never the pull request: its author
 * controls the head, and a change to an instruction is itself one of the changes reviewed. */
function baseInstructions(
  baseSha: string,
  itemPaths: string[],
): { blocks: { instruction: FolderInstruction; body: string }[]; skipped: string[] } {
  let listing = "";
  try {
    listing = git([
      "grep",
      "-l",
      "-E",
      `resource_type['"]?:[[:space:]]*['"]?${REVIEW_INSTRUCTIONS_RESOURCE_TYPE}`,
      baseSha,
      "--",
      "*.resource.yaml",
      "*.resource.json",
    ]);
  } catch (e) {
    // `git grep` exits 1 when nothing matches.
    if ((e as { status?: number }).status !== 1) throw e;
  }
  const listed = listing
    .split("\n")
    .filter(Boolean)
    .map((line) => line.slice(baseSha.length + 1))
    .map((file) => {
      const path = file.replace(/\.resource\.(yaml|json)$/, "");
      return { file, instruction: { path, scope: folderInstructionScope(path) } };
    });
  const fileOf = new Map(listed.map((l) => [l.instruction.path, l.file]));
  const blocks: { instruction: FolderInstruction; body: string }[] = [];
  const skipped: string[] = [];
  for (const instruction of instructionsCovering(listed.map((l) => l.instruction), itemPaths)) {
    const file = fileOf.get(instruction.path)!;
    try {
      const raw = git(["show", `${baseSha}:${file}`]);
      const parsed = (file.endsWith(".json") ? JSON.parse(raw) : yamlParseContent(file, raw)) as {
        resource_type?: unknown;
        value?: { content?: unknown };
      };
      const content = parsed.value?.content;
      if (parsed.resource_type !== REVIEW_INSTRUCTIONS_RESOURCE_TYPE) continue;
      if (typeof content !== "string") throw new Error("no string content");
      const inline = /^!inline (.+)$/.exec(content);
      blocks.push({
        instruction,
        body: inline ? git(["show", `${baseSha}:${inline[1].trim()}`]) : content,
      });
    } catch (e) {
      log.warn(`Could not read review instructions ${instruction.path}: ${e}`);
      skipped.push(instruction.path);
    }
  }
  return { blocks, skipped };
}

/** A synced workspace encryption key is plaintext, so it is withheld like a secret value. */
const WITHHELD_KINDS = new Set([...VALUE_WITHHELD_KINDS, "encryption_key"]);

/** Instruction resources hold prose rather than credentials, so a change to one is shown to
 * the model like any other: a pull request rewriting the review's own instructions is
 * something the reviewer should hear about. */
const INSTRUCTION_RESOURCE_TYPES = new Set([REVIEW_INSTRUCTIONS_RESOURCE_TYPE, "ai_instruction"]);

/** Whether `item` is an instruction resource on every revision that defines it. Each side is
 * judged on its own: a resource retyped on either side holds credentials on the other. Any
 * changed file beside the resource's own definition and body (a workspace-specific variant, a
 * fileset child) may hold credentials of its own, so it withholds the whole item. */
function isInstructionResource(item: ChangedItem, revs: string[]): boolean {
  if (item.kind !== "resource") return false;
  const ownFile = (file: string) => {
    if (!file.startsWith(`${item.path}.resource.`)) return false;
    const rest = file.slice(item.path.length + ".resource.".length);
    return rest === "yaml" || rest === "json" || /^file\.[^./]+$/.test(rest);
  };
  if (!item.files.every((f) => ownFile(f.file))) return false;
  let defined = false;
  for (const rev of revs) {
    for (const file of [`${item.path}.resource.yaml`, `${item.path}.resource.json`]) {
      let raw: string;
      try {
        raw = git(["show", `${rev}:${file}`]);
      } catch {
        continue;
      }
      try {
        const parsed = (file.endsWith(".json") ? JSON.parse(raw) : yamlParseContent(file, raw)) as {
          resource_type?: unknown;
        };
        if (!INSTRUCTION_RESOURCE_TYPES.has(parsed.resource_type as string)) return false;
      } catch {
        return false;
      }
      defined = true;
    }
  }
  return defined;
}

function parseModel(model: string | undefined) {
  if (!model) return undefined;
  const i = model.indexOf(":");
  if (i <= 0) throw new Error(`--model takes provider:model, got ${model}`);
  return { provider: model.slice(0, i), model: model.slice(i + 1) } as any;
}

/** What the model is asked, and what its answer may anchor to, for the changes `headSha`
 * makes on top of its merge base with `baseSha`. Reads the repository in the working
 * directory. */
export function buildReviewPrompt(baseSha: string, headSha: string) {
  let mergeBase: string;
  try {
    mergeBase = git(["merge-base", baseSha, headSha]).trim();
  } catch {
    throw new Error(
      `No common ancestor between ${baseSha} and ${headSha}. Fetch the full history (actions/checkout with fetch-depth: 0).`,
    );
  }
  const items = changedItems(mergeBase, headSha);
  const { blocks, skipped } = baseInstructions(
    baseSha,
    items.map((i) => i.path),
  );

  const budget = new ReviewBudget();
  const unreadable: string[] = [];
  const lines = new Map<string, { LEFT: Set<number>; RIGHT: Set<number> }>();
  for (const item of items) {
    const key = `${item.kind}:${item.path}`;
    if (budget.spent) {
      budget.omitted.push(key);
      continue;
    }
    const header = `### ${item.kind} ${item.path} (${changeOf(item)})`;
    if (WITHHELD_KINDS.has(item.kind) && !isInstructionResource(item, [mergeBase, headSha])) {
      budget.add(key, `${header}\nValue withheld from review.`);
      continue;
    }
    const files: string[] = [];
    let failed = false;
    for (const { file, status } of item.files) {
      const shown = numberDiff(
        git(["diff", "--no-color", "--no-ext-diff", `-U${MODEL_DIFF_CONTEXT}`, mergeBase, headSha, "--", `:(literal)${file}`]),
      );
      const commentable = numberDiff(
        git(["diff", "--no-color", "--no-ext-diff", `-U${GITHUB_DIFF_CONTEXT}`, mergeBase, headSha, "--", `:(literal)${file}`]),
      );
      if (!shown || !commentable) {
        failed = true;
        files.push(`#### ${file} (${status})\nContents could not be read.`);
        continue;
      }
      lines.set(file, { LEFT: commentable.left, RIGHT: commentable.right });
      files.push(`#### ${file} (${status})\n${capText(shown.text, 2 * PER_SIDE_CHAR_CAP)}`);
    }
    if (failed) unreadable.push(key);
    budget.add(key, `${header}\n${files.join("\n")}`);
  }

  return {
    system: reviewSystemPrompt("pull_request") + reviewInstructionsSection("pull_request", blocks),
    user: budget.userPrompt(`Base: ${baseSha}\nPull request head: ${headSha}`),
    anchors: { items: new Set(items.map((i) => `${i.kind}:${i.path}`)), lines },
    omitted: budget.omitted,
    unreadable,
    instructions: blocks.map((b) => b.instruction.path),
    skippedInstructions: skipped,
  };
}

export async function reviewChanges(
  workspaceId: string,
  model: string | undefined,
  baseSha: string,
  headSha: string,
): Promise<ReviewResult> {
  const { system, user, anchors, ...reported } = buildReviewPrompt(baseSha, headSha);
  const response = await wmill.completeAi({
    workspace: workspaceId,
    requestBody: { system, user, model: parseModel(model), max_tokens: MAX_OUTPUT_TOKENS },
  });
  return {
    ...parseReviewAnswer(response.text, anchors),
    ...reported,
    model: `${response.model.provider}:${response.model.model}`,
  };
}

const SEVERITY_RANK: Record<AiReviewSeverity, number> = { high: 0, medium: 1, low: 2 };

function findingLine(f: AiReviewFinding): string {
  const where = [f.path && `\`${f.path}\``, f.file && f.file !== f.path && `\`${f.file}\``]
    .filter(Boolean)
    .join(" ");
  return `- **${f.severity}**${where ? ` ${where}` : ""}: ${f.comment}`;
}

/** The review body: everything a line comment does not carry. */
export function reviewBody(
  result: ReviewResult,
  marker: string,
  inBody: AiReviewFinding[],
): string {
  const sections = [marker, "### Windmill AI review", result.summary || "No summary."];
  if (result.findings.length === 0) sections.push("No issues found.");
  if (inBody.length > 0) sections.push(inBody.map(findingLine).join("\n"));
  const list = (title: string, entries: string[]) =>
    entries.length > 0 && sections.push(`${title}\n${entries.map((e) => `- \`${e}\``).join("\n")}`);
  list("**Not reviewed**, the files could not be read as text:", result.unreadable);
  list("**Reviewed by name only**, over the size budget:", result.omitted);
  list("Review instructions applied, from the base branch:", result.instructions);
  list("**Review instructions skipped**, their body could not be read:", result.skippedInstructions);
  sections.push(`<sub>${result.model}</sub>`);
  return sections.join("\n\n");
}

type GitHubContext = { api: string; repo: string; token: string; pr: number };

async function github(ctx: GitHubContext, method: string, path: string, body?: unknown) {
  const res = await fetch(`${ctx.api}/repos/${ctx.repo}${path}`, {
    method,
    headers: {
      Authorization: `Bearer ${ctx.token}`,
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
      ...(body ? { "Content-Type": "application/json" } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = new Error(`GitHub ${method} ${path}: ${res.status} ${await res.text()}`);
    (err as { status?: number }).status = res.status;
    throw err;
  }
  return res.json();
}

async function alreadyReviewed(ctx: GitHubContext, marker: string): Promise<boolean> {
  for (let page = 1; page <= 20; page++) {
    const reviews = (await github(
      ctx,
      "GET",
      `/pulls/${ctx.pr}/reviews?per_page=100&page=${page}`,
    )) as { body?: string }[];
    if (reviews.some((r) => r.body?.includes(marker))) return true;
    if (reviews.length < 100) return false;
  }
  return false;
}

/**
 * What a pull request gets: a review whose line comments carry the anchored findings, and the
 * body to post instead if the host refuses them. Printed with `--json`, so a server posting the
 * review through its own credential sends exactly what `--post github` would.
 */
export function reviewPayload(result: ReviewResult, headSha: string) {
  const marker = `<!-- windmill-ai-review:${headSha} -->`;
  const anchored = result.findings.filter((f) => f.file && f.line);
  return {
    /** One review per head commit: a review whose body carries this one is already posted. */
    marker,
    body: reviewBody(result, marker, result.findings.filter((f) => !(f.file && f.line))),
    comments: anchored.map((f) => ({
      path: f.file!,
      line: f.line!,
      side: f.side!,
      body: `**${f.severity}**: ${f.comment}`,
    })),
    /** Every finding in the body, for a host that refuses a comment or has no line comments. */
    fallback_body: reviewBody(result, marker, result.findings),
  };
}

async function postReview(ctx: GitHubContext, headSha: string, result: ReviewResult) {
  const { marker, body, comments, fallback_body } = reviewPayload(result, headSha);
  if (await alreadyReviewed(ctx, marker)) {
    log.info(`This commit already has a Windmill AI review, not posting another.`);
    return;
  }
  try {
    await github(ctx, "POST", `/pulls/${ctx.pr}/reviews`, {
      commit_id: headSha,
      event: "COMMENT",
      body,
      comments,
    });
  } catch (e) {
    // A comment GitHub cannot place fails the whole review, so the findings go in its body.
    if ((e as { status?: number }).status !== 422 || comments.length === 0) throw e;
    log.warn(`GitHub refused the line comments, posting them in the review body: ${e}`);
    await github(ctx, "POST", `/pulls/${ctx.pr}/reviews`, {
      commit_id: headSha,
      event: "COMMENT",
      body: fallback_body,
    });
  }
  log.info(colors.green(`Posted the review on pull request #${ctx.pr}.`));
}

function githubEvent(): any {
  const path = process.env.GITHUB_EVENT_PATH;
  if (!path) return undefined;
  try {
    return JSON.parse(readFileSync(path, "utf-8"));
  } catch {
    return undefined;
  }
}

function githubContext(opts: ReviewOptions, event: any): GitHubContext {
  const token = process.env.GITHUB_TOKEN;
  const repo = process.env.GITHUB_REPOSITORY;
  const pr = opts.pr ?? event?.pull_request?.number;
  if (!token || !repo || !pr) {
    throw new Error(
      "--post github needs GITHUB_TOKEN and GITHUB_REPOSITORY in the environment, and the pull request number (--pr, or a pull_request event)",
    );
  }
  return { api: process.env.GITHUB_API_URL ?? "https://api.github.com", repo, token, pr };
}

function printResult(result: ReviewResult) {
  log.info(colors.bold(result.summary || "No summary."));
  for (const f of result.findings) {
    const where = [f.path, f.file && f.line ? `${f.file}:${f.side === "LEFT" ? "L" : "R"}${f.line}` : f.file]
      .filter(Boolean)
      .join(" ");
    console.log(`${f.severity.toUpperCase()} ${where}\n  ${f.comment}`);
  }
  if (result.findings.length === 0) log.info("No issues found.");
  if (result.unreadable.length) log.warn(`Not reviewed, unreadable: ${result.unreadable.join(", ")}`);
  if (result.omitted.length) log.warn(`Reviewed by name only, over budget: ${result.omitted.join(", ")}`);
  if (result.instructions.length) log.info(`Review instructions applied: ${result.instructions.join(", ")}`);
  if (result.skippedInstructions.length) {
    log.warn(`Review instructions skipped: ${result.skippedInstructions.join(", ")}`);
  }
}

async function review(opts: ReviewOptions) {
  if (opts.json) log.setSilent(true);
  // The checkout is the pull request's head, which its author controls, so nothing is read
  // from its wmill.yaml or the local profiles it could select: the instance, workspace and
  // token come from the command line only, and the token never reaches a URL a branch names.
  if (!opts.baseUrl || !opts.workspace || !opts.token) {
    throw new Error("wmill review needs --base-url, --workspace and --token");
  }
  const event = githubEvent();
  const base = opts.base ?? event?.pull_request?.base?.sha;
  const head = opts.head ?? event?.pull_request?.head?.sha ?? "HEAD";
  if (!base) throw new Error("--base is required outside a GitHub pull_request event");
  const baseSha = git(["rev-parse", "--verify", `${base}^{commit}`]).trim();
  const headSha = git(["rev-parse", "--verify", `${head}^{commit}`]).trim();
  const ctx = opts.post === "github" ? githubContext(opts, event) : undefined;

  await requireLogin(opts);
  const { workspaceId } = await resolveWorkspace(opts);
  let result: ReviewResult;
  try {
    result = await reviewChanges(workspaceId, opts.model, baseSha, headSha);
  } catch (e) {
    if (ctx) {
      await github(ctx, "POST", `/pulls/${ctx.pr}/reviews`, {
        commit_id: headSha,
        event: "COMMENT",
        body: `### Windmill AI review\n\nThe review could not be completed: ${apiErrorMessage(e) ?? (e instanceof Error ? e.message : e)}`,
      }).catch((postError) => log.error(`Could not report the failure on the pull request: ${postError}`));
    }
    throw e;
  }

  if (opts.json) console.log(JSON.stringify({ ...result, post: reviewPayload(result, headSha) }));
  else printResult(result);
  if (ctx) await postReview(ctx, headSha, result);
  if (
    opts.failOn &&
    result.findings.some((f) => SEVERITY_RANK[f.severity] <= SEVERITY_RANK[opts.failOn!])
  ) {
    process.exitCode = 1;
  }
}

const command = new Command()
  .description(
    "Review a pull request on a git sync repository with the workspace's AI, the way deployment requests are reviewed. Reads the changes and the base branch's review instructions from git; never reads the checkout's wmill.yaml.",
  )
  .option("--base <ref:string>", "The base branch commit. Defaults to the pull_request event's base.")
  .option("--head <ref:string>", "The pull request's commit. Defaults to the event's head, else HEAD.")
  .option("--model <model:string>", "provider:model to review with. Defaults to the workspace's default model.")
  .option("--json", "Print the review as JSON")
  .option("--post <target:string>", "Post the review on the pull request. Only `github` is supported.", {
    value: (v: string) => {
      if (v !== "github") throw new Error(`--post supports github only, got ${v}`);
      return v as "github";
    },
  })
  .option("--pr <number:number>", "The pull request number. Defaults to the pull_request event's.")
  .option("--fail-on <severity:string>", "Exit with 1 when a finding is at least this severe: high, medium or low.", {
    value: (v: string) => {
      if (!["high", "medium", "low"].includes(v)) throw new Error(`--fail-on takes high, medium or low, got ${v}`);
      return v as AiReviewSeverity;
    },
  })
  .action(review as any);

export default command;
