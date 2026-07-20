import * as log from "../core/log.ts";
import { execSync, spawnSync } from "node:child_process";
import { WM_FORK_PREFIX } from "../core/constants.ts";

// Fork *workspace id* prefix ("wm-fork-"). WM_FORK_PREFIX is the *branch*
// prefix ("wm-fork") used inside the wm-fork/<branch>/<id> branch name.
const FORK_WORKSPACE_PREFIX = `${WM_FORK_PREFIX}-`;

export function getCurrentGitBranch(): string | null {
  try {
    const result = execSync("git rev-parse --abbrev-ref HEAD", {
      encoding: "utf8",
      stdio: "pipe"
    });
    const branch = result.trim();
    return branch || null;
  } catch (error) {
    log.debug(`Failed to get Git branch: ${error}`);
    return null;
  }
}

/**
 * The push URL of the given git remote (default `origin`), or null if unset /
 * not a git repo. Uses `--push` since deploy-on-push is about where `git push`
 * lands (a remote may set a distinct pushurl). Argument-based (no shell) so a
 * caller-supplied remote name can't inject a command.
 */
export function getGitRemoteUrl(remote = "origin"): string | null {
  const r = spawnSync("git", ["remote", "get-url", "--push", remote], {
    encoding: "utf8",
    stdio: "pipe",
  });
  if (r.status !== 0) {
    log.debug(`Failed to get Git remote url: ${r.stderr}`);
    return null;
  }
  const url = r.stdout.trim();
  return url || null;
}

/**
 * Remove any embedded `user:token@` credentials from a git remote URL. Sent to
 * the backend for matching, so the token must never leave the machine (it would
 * otherwise land in request-URI logs). scp-like `git@host:path` carries no token
 * (key auth) and is returned unchanged.
 */
export function stripGitRemoteCredentials(url: string): string {
  try {
    const u = new URL(url);
    u.username = "";
    u.password = "";
    return u.toString();
  } catch {
    return url;
  }
}

/**
 * POSIX single-quote a string so it is safe to interpolate into a shell command
 * we recommend to a user or agent. Wraps in `'...'` and escapes embedded quotes.
 */
export function shellQuote(s: string): string {
  return `'${s.replace(/'/g, "'\\''")}'`;
}

/** Whether a local branch with this exact name exists. */
export function gitBranchExists(branchName: string): boolean {
  const r = spawnSync(
    "git",
    ["show-ref", "--verify", "--quiet", `refs/heads/${branchName}`],
    { stdio: "pipe" },
  );
  return r.status === 0;
}

/**
 * Rename the currently checked-out branch (`git branch -m <newName>`). Used by
 * `wmill workspace fork --from-branch` to turn an existing working branch into
 * the `wm-fork/<base>/<id>` fork branch in place, preserving its commits.
 */
export function renameCurrentGitBranch(newName: string): void {
  const r = spawnSync("git", ["branch", "-m", newName], {
    encoding: "utf8",
    stdio: "pipe",
  });
  if ((r.status ?? 1) !== 0) {
    throw new Error(
      `git branch -m ${newName} failed (exit ${r.status}): ${r.stderr ?? ""}`,
    );
  }
}

export function getOriginalBranchForWorkspaceForks(branchName: string | null): string | null {
  if (!branchName || !branchName.startsWith(WM_FORK_PREFIX)) {
    return null
  }

  const start = branchName.indexOf("/") + 1;
  const end = branchName.lastIndexOf("/");

  if (start < 0 || end < 0 || end - start <= 0) {
    return null
  }

  return branchName.slice(start, end)
}

export function getWorkspaceIdForWorkspaceForkFromBranchName(branchName: string): string | null {
  if (!branchName.startsWith(WM_FORK_PREFIX)) {
    return null
  }

  const start = branchName.lastIndexOf("/") + 1;

  if (start < 0) {
    return null
  }

  return `${WM_FORK_PREFIX}-${branchName.slice(start)}`
}
export function isGitRepository(): boolean {
  try {
    execSync("git rev-parse --git-dir", {
      encoding: "utf8",
      stdio: "pipe"
    });
    return true;
  } catch (error) {
    log.debug(`Failed to check Git repository: ${error}`);
    return false;
  }
}

// ===========================================================================
// Git-sync deployment-callback branch/commit/push.
//
// The git-sync hub script ("sync-script-to-git-repo-windmill") historically
// owned this logic. When `use_individual_branch` is set, a deploy must land on
// a dedicated `wm_deploy/<workspace>/<...>` branch (or a `wm-fork/<branch>/<id>`
// branch for forks) instead of the repo's protected base branch — otherwise
// the push to a protected `main` is rejected (GH006). A hub-script rewrite once
// dropped this and every deploy pushed straight to `main`; keeping the logic
// here (in-repo, behind `wmill sync pull`) makes it deterministically testable.
// ===========================================================================

export interface GitSyncDeployItem {
  path_type: string;
  path?: string | null;
  parent_path?: string | null;
  commit_msg?: string;
}

// A fork or dev workspace syncs to its own wm-fork/<branch>/<id> branch. The hub
// script force-disables use_individual_branch / group_by_folder for these (that
// disabling also changes the include/promotion derivation — so callers must apply
// it BEFORE deriving includes, not just for branch naming).
//
// Fork-ness is "has a parent workspace" OR the "wm-fork-" id prefix. Regular forks
// get an auto-generated `wm-fork-<slug>` id, but dev workspaces keep a custom id
// with no prefix — so the prefix alone misses them. Mirrors the backend's
// `parent.is_some() || starts_with(WM_FORK_PREFIX)` rule (the prefix also covers a
// fork whose parent was deleted, since parent_workspace_id is ON DELETE SET NULL).
export function isForkWorkspace(
  workspaceId: string,
  parentWorkspaceId?: string | null,
): boolean {
  return !!parentWorkspaceId || workspaceId.startsWith(FORK_WORKSPACE_PREFIX);
}

// Mirrors the hub script's get_fork_branch_name. A dev workspace syncs with its
// environment-label branch verbatim ("dev"/"staging" — a first-class top-level
// branch; the backend passes the label with the deploy). A `wm-fork-<slug>`
// throwaway fork id becomes "wm-fork/<originalBranch>/<slug>"; a prefix-less id
// without a label falls back to "wm-fork/<originalBranch>/<id>".
export function forkBranchName(
  workspaceId: string,
  originalBranch: string,
  devWorkspaceLabel?: string | null,
): string {
  if (devWorkspaceLabel) {
    return devWorkspaceLabel;
  }
  const branchPrefix = `${WM_FORK_PREFIX}/${originalBranch}/`;
  return workspaceId.startsWith(FORK_WORKSPACE_PREFIX)
    ? workspaceId.replace(FORK_WORKSPACE_PREFIX, branchPrefix)
    : `${branchPrefix}${workspaceId}`;
}

// Pure branch-name resolution mirroring the hub script's git_checkout_branch.
// Returns null when the deploy should stay on the cloned/base branch (i.e.
// workspace-wide mode, or user/group objects which never get their own branch).
export function computeGitSyncDeployBranch(params: {
  workspaceId: string;
  parentWorkspaceId?: string | null;
  devWorkspaceLabel?: string | null;
  items: GitSyncDeployItem[];
  useIndividualBranch: boolean;
  groupByFolder: boolean;
  clonedBranchName: string;
}): string | null {
  const {
    workspaceId,
    parentWorkspaceId,
    devWorkspaceLabel,
    items,
    useIndividualBranch,
    groupByFolder,
    clonedBranchName,
  } = params;

  if (isForkWorkspace(workspaceId, parentWorkspaceId)) {
    return forkBranchName(workspaceId, clonedBranchName, devWorkspaceLabel);
  }

  if (items.length === 0) return null;
  const first = items[0];

  // `use_individual_branch` disables debouncing, so items is length 1 here.
  if (
    !useIndividualBranch ||
    first.path_type === "user" ||
    first.path_type === "group"
  ) {
    return null;
  }

  const ref = first.path ?? first.parent_path;
  if (!ref) return null;

  return groupByFolder
    ? `wm_deploy/${workspaceId}/${ref.split("/").slice(0, 2).join("__")}`
    : `wm_deploy/${workspaceId}/${first.path_type}/${ref.replaceAll("/", "__")}`;
}

// Mirrors the hub script's composeCommitHeader: summarise the deployed object
// types, e.g. "[WM]: Deployed 2 scripts, 1 flow and 3 other objects".
export function composeGitSyncCommitHeader(
  items: GitSyncDeployItem[],
): string {
  const typeCounts = new Map<string, number>();
  for (const item of items) {
    typeCounts.set(item.path_type, (typeCounts.get(item.path_type) ?? 0) + 1);
  }
  const sorted = Array.from(typeCounts.entries()).sort((a, b) => b[1] - a[1]);

  const parts: string[] = [];
  let othersCount = 0;
  for (let i = 0; i < sorted.length; i++) {
    const [pathType, count] = sorted[i];
    if (i < 3) {
      const label = count > 1 ? `${pathType}s` : pathType;
      if (i === 2 && sorted.length === 3) {
        parts.push(`and ${count} ${label}`);
      } else {
        parts.push(`${count} ${label}`);
      }
    } else {
      othersCount += count;
    }
  }

  let header = `[WM]: Deployed ${parts.join(", ")}`;
  if (othersCount > 0) {
    header += ` and ${othersCount} other object${othersCount > 1 ? "s" : ""}`;
  }
  return header;
}

// Mirrors the hub script's git_push commit-message construction EXACTLY,
// including its quirks: it pushes `commit_msg` once per item unconditionally
// (so the array length equals items.length even when some msgs are empty),
// uses the single message verbatim when there is exactly one item, otherwise
// the composed header + newline-joined descriptions, and falls back to
// "no commit msg" when the chosen header is empty/undefined. join() coerces
// undefined entries to "" just like the original.
export function gitSyncCommitMessage(items: GitSyncDeployItem[]): {
  header: string;
  description: string;
} {
  const descs = items.map((i) => i.commit_msg);
  const [h, d] =
    descs.length === 1
      ? [descs[0], ""]
      : [composeGitSyncCommitHeader(items), descs.join("\n")];
  return {
    header: h === undefined || h === "" ? "no commit msg" : h,
    description: d ?? "",
  };
}

// Mirrors the hub script's regexFromPath: the include glob(s) that select an
// object's files for `wmill sync pull --extra-includes`. Some types expand to
// two comma-separated patterns (dotted + double-underscore folder layouts).
export function gitSyncIncludePattern(
  pathType: string,
  path: string,
): string {
  switch (pathType) {
    case "flow":
      return `${path}.flow/*,${path}__flow/*`;
    case "app":
      return `${path}.app/*,${path}__app/*`;
    case "raw_app":
      return `${path}.raw_app/**,${path}__raw_app/**`;
    case "folder":
      return `${path}/folder.meta.*`;
    case "resourcetype":
      return `${path}.resource-type.*`;
    case "resource":
      return `${path}.resource.*`;
    case "variable":
      return `${path}.variable.*`;
    case "schedule":
      return `${path}.schedule.*`;
    case "user":
      return `${path}.user.*`;
    case "group":
      return `${path}.group.*`;
    case "httptrigger":
      return `${path}.http_trigger.*`;
    case "websockettrigger":
      return `${path}.websocket_trigger.*`;
    case "kafkatrigger":
      return `${path}.kafka_trigger.*`;
    case "natstrigger":
      return `${path}.nats_trigger.*`;
    case "postgrestrigger":
      return `${path}.postgres_trigger.*`;
    case "mqtttrigger":
      return `${path}.mqtt_trigger.*`;
    case "sqstrigger":
      return `${path}.sqs_trigger.*`;
    case "gcptrigger":
      return `${path}.gcp_trigger.*`;
    case "azuretrigger":
      return `${path}.azure_trigger.*`;
    case "emailtrigger":
      return `${path}.email_trigger.*`;
    default:
      // Scripts: `${path}.*` matches the dotted layout
      // (`${path}.script.yaml` etc.), `${path}__mod/**` matches the folder
      // layout used by scripts with companion modules
      // (`${path}__mod/script.ts`, `${path}__mod/helper.ts`, ...). Without the
      // second pattern the module files are filtered out of the pull and the
      // subsequent `git add '${path}**'` fails with "pathspec did not match".
      return `${path}.*,${path}__mod/**`;
  }
}

// `forcedIncludes` carries ONLY the include-* flags that must be force-set to
// true (overriding the repo's wmill.yaml). Kinds not present are intentionally
// omitted (never set to false) so the caller can spread this object and let
// the repo's effective config govern the rest — see deriveGitSyncDeployIncludes.
export type GitSyncForcedIncludes = Partial<{
  includeSchedules: boolean;
  includeGroups: boolean;
  includeUsers: boolean;
  includeTriggers: boolean;
  includeSettings: boolean;
  includeKey: boolean;
}>;

export interface GitSyncDeployIncludes {
  extraIncludes: string[];
  forcedIncludes: GitSyncForcedIncludes;
}

// Mirrors the hub script's wmill_sync_pull include-derivation: build the
// --extra-includes set from the deployed items, and decide which default-
// excluded object kinds (triggers, schedules, groups, users, settings, key)
// must be force-included in the pull. Replaces the script's regexFromPath +
// per-kind --include-* construction so the hub script can drop both.
//
// Branch-mode distinction (this is load-bearing — see the trigger-promotion
// bug it fixes):
//   - Workspace-wide mode: the repo is a full mirror of the workspace, so a
//     deployed object of a default-excluded kind MUST be re-included, even if
//     wmill.yaml would otherwise skip it. We force the flag on.
//   - Individual-branch (promotion) mode: the repo is a filtered prod surface
//     whose own wmill.yaml filters decide what gets promoted. We force NOTHING
//     here and the keys stay absent, so the caller's pull resolves them from
//     the target's effective config (a deployed trigger lands iff the target
//     includes triggers). Forcing `false` (the original behavior) did NOT
//     defer — it CLOBBERED the effective config via Object.assign in pull's
//     option merge, silently dropping kinds the target actually wanted (e.g. a
//     deployed trigger when the target has includeTriggers: true), and the
//     server then omitted the object from the tarball entirely.
export function deriveGitSyncDeployIncludes(
  items: GitSyncDeployItem[],
  useIndividualBranch: boolean,
): GitSyncDeployIncludes {
  const extraIncludes: string[] = [];
  for (const { path_type, path, parent_path } of items) {
    if (path) {
      extraIncludes.push(...gitSyncIncludePattern(path_type, path).split(","));
    }
    if (parent_path) {
      extraIncludes.push(
        ...gitSyncIncludePattern(path_type, parent_path).split(","),
      );
    }
  }

  const forcedIncludes: GitSyncForcedIncludes = {};
  if (!useIndividualBranch) {
    const has = (pred: (t: string) => boolean) =>
      items.some((i) => pred(i.path_type));
    if (has((t) => t === "schedule")) forcedIncludes.includeSchedules = true;
    if (has((t) => t === "group")) forcedIncludes.includeGroups = true;
    if (has((t) => t === "user")) forcedIncludes.includeUsers = true;
    if (has((t) => t.includes("trigger"))) forcedIncludes.includeTriggers = true;
    if (has((t) => t === "settings")) forcedIncludes.includeSettings = true;
    if (has((t) => t === "key")) forcedIncludes.includeKey = true;
  }

  return { extraIncludes, forcedIncludes };
}

function git(
  args: string[],
  opts?: { allowFail?: boolean },
): { status: number; stdout: string; stderr: string } {
  const r = spawnSync("git", args, { encoding: "utf8", stdio: "pipe" });
  const status = r.status ?? 1;
  if (r.error) {
    if (opts?.allowFail) return { status, stdout: "", stderr: String(r.error) };
    throw r.error;
  }
  if (status !== 0 && !opts?.allowFail) {
    throw new Error(
      `git ${args.join(" ")} failed (exit ${status}): ${r.stderr ?? ""}`,
    );
  }
  return { status, stdout: r.stdout ?? "", stderr: r.stderr ?? "" };
}

// Checkout (or create) the dedicated deploy branch, mirroring the hub script:
// try `git checkout <branch>`, on failure create it with -b and enable
// push.autoSetupRemote so the subsequent bare `git push` targets it.
export function checkoutGitSyncDeployBranch(branch: string): void {
  const existing = git(["checkout", branch], { allowFail: true });
  if (existing.status === 0) {
    log.info(`Switched to existing branch ${branch}`);
    return;
  }
  git(["checkout", "-b", branch]);
  git(["config", "--add", "--bool", "push.autoSetupRemote", "true"]);
  log.info(`Created and switched to branch ${branch}`);
}

// Stage, commit and push the deploy, mirroring the hub script's git_push:
// stage `wmill-lock.yaml` plus each item's path tree, no-op when nothing is
// staged, and push the *current* branch with a rebase-retry fallback.
export function gitSyncDeployPush(params: {
  items: GitSyncDeployItem[];
  authorName: string;
  authorEmail: string;
  // Committer identity (git config user.*). Defaults to the author identity,
  // matching the hub script's non-GPG branch. For GPG-signed repos the hub
  // sets the committer email to the GPG key's email — the caller passes that
  // through here so the committed identity stays 1:1.
  committerName?: string;
  committerEmail?: string;
  onlyCreateBranch?: boolean;
}): { pushed: boolean } {
  const { items, authorName, authorEmail, onlyCreateBranch } = params;
  const committerName = params.committerName ?? authorName;
  const committerEmail = params.committerEmail ?? authorEmail;

  git(["config", "user.email", committerEmail]);
  git(["config", "user.name", committerName]);

  // only_create_branch: publish the (possibly empty) branch ref and stop.
  // Mirrors the hub script's early `git push --porcelain` (NOT swallowed —
  // a failure here fails the job, exactly as in the hub).
  if (onlyCreateBranch) {
    git(["push", "--porcelain"]);
    return { pushed: true };
  }

  // 1:1 with the hub script: `git add wmill-lock.yaml <path>**` as a single
  // command per item, failure swallowed (its try/catch). `commit_msg` is
  // collected once per item unconditionally — see gitSyncCommitMessage.
  for (const { path, parent_path } of items) {
    if (path) {
      git(["add", "wmill-lock.yaml", `${path}**`], { allowFail: true });
    }
    if (parent_path) {
      git(["add", "wmill-lock.yaml", `${parent_path}**`], { allowFail: true });
    }
  }

  // `git diff --cached --quiet` exits 1 iff there is something staged.
  const staged = git(["diff", "--cached", "--quiet"], { allowFail: true });
  if (staged.status === 0) {
    log.info("No changes detected, nothing to commit.");
    return { pushed: false };
  }

  const { header, description } = gitSyncCommitMessage(items);

  git([
    "commit",
    "--author",
    `${authorName} <${authorEmail}>`,
    "-m",
    header,
    "-m",
    description,
  ]);

  const push = git(["push", "--porcelain"], { allowFail: true });
  if (push.status !== 0) {
    log.info(`Push failed, rebasing and retrying: ${push.stderr}`);
    git(["pull", "--rebase"]);
    git(["push", "--porcelain"]);
  }
  return { pushed: true };
}
