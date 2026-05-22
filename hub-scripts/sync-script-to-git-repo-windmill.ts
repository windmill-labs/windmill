// Source-of-truth copy of the git-sync deployment-callback hub script.
// Published to hub.windmill.dev as `sync-script-to-git-repo-windmill`; the
// hub id this corresponds to is referenced in
// `backend/windmill-common/src/workspaces.rs::LATEST_GIT_SYNC_SCRIPT_PATH`.
//
// Diff vs. hub/28231 (WIN-1974 fix): the agent-cache pre-warm (`gpg -bsau`
// with --passphrase) is replaced by a stateless `gpg.program` wrapper script
// + chmod-600 passphrase file. Git invokes the wrapper on every sign, the
// wrapper always uses `--pinentry-mode loopback` (+ `--passphrase-file` when
// a passphrase exists), so signing does NOT depend on gpg-agent having a
// cached passphrase by the time the CLI's `git commit` runs minutes later.
import * as wmillclient from "windmill-client";

import wmill from "windmill-cli@1.705.0";
import { basename } from "node:path";
const util = require("util");
const exec = util.promisify(require("child_process").exec);
import process from "process";

type GpgKey = {
  email: string;
  private_key: string;
  passphrase: string;
};

const FORKED_WORKSPACE_PREFIX = "wm-fork-";
const GPG_HOME = "/tmp/gpg";
const GPG_PASSPHRASE_FILE = `${GPG_HOME}/passphrase`;
const GPG_WRAPPER = `${GPG_HOME}/gpg-wrapper.sh`;

type PathType =
  | "script"
  | "flow"
  | "app"
  | "raw_app"
  | "folder"
  | "resource"
  | "variable"
  | "resourcetype"
  | "schedule"
  | "user"
  | "group"
  | "httptrigger"
  | "websockettrigger"
  | "kafkatrigger"
  | "natstrigger"
  | "postgrestrigger"
  | "mqtttrigger"
  | "sqstrigger"
  | "gcptrigger"
  | "azuretrigger"
  | "emailtrigger";

type SyncObject = {
  path_type: PathType;
  path: string | undefined;
  parent_path: string | undefined;
  commit_msg: string;
};

let gpgFingerprint: string | undefined = undefined;
let gpgWrapperWritten = false;

export async function main(
  items: SyncObject[],
  // Compat, do not use in code, rely on `items` instead
  path_type: PathType | undefined,
  path: string | undefined,
  parent_path: string | undefined,
  commit_msg: string | undefined,
  //
  workspace_id: string,
  repo_url_resource_path: string,
  skip_secret: boolean = true,
  use_individual_branch: boolean = false,
  group_by_folder: boolean = false,
  only_create_branch: boolean = false,
  parent_workspace_id?: string,
) {
  if (path_type !== undefined && commit_msg !== undefined) {
    items = [
      {
        path_type,
        path,
        parent_path,
        commit_msg,
      },
    ];
  }
  await inner(
    items,
    workspace_id,
    repo_url_resource_path,
    skip_secret,
    use_individual_branch,
    group_by_folder,
    only_create_branch,
    parent_workspace_id,
  );
}

// Thin git-sync deployment-callback script.
//
// All git orchestration (wm_deploy/fork branch selection, include/promotion
// derivation, commit, push, fork-disable, parent-of-fork rooting) lives in
// the bundled CLI's hidden `wmill sync git-deploy`. This script only does the
// parts that need Windmill resource/secret access and must wrap the clone:
// resolve the repo resource, GitHub-App / Azure auth, `git clone`, GPG key
// import + wrapper config, then delegate, then clean up.
async function inner(
  items: SyncObject[],
  workspace_id: string,
  repo_url_resource_path: string,
  skip_secret: boolean = true,
  use_individual_branch: boolean = false,
  group_by_folder: boolean = false,
  only_create_branch: boolean = false,
  parent_workspace_id?: string,
) {
  let safeDirectoryPath: string | undefined;
  const repo_resource = await wmillclient.getResource(repo_url_resource_path);
  const cwd = process.cwd();
  process.env["HOME"] = ".";
  if (!only_create_branch) {
    for (const item of items) {
      console.log(
        `Syncing ${item.path_type} ${item.path ?? ""} with parent ${item.parent_path ?? ""}`,
      );
    }
  }

  if (repo_resource.is_github_app) {
    const token = await get_gh_app_token();
    const authRepoUrl = prependTokenToGitHubUrl(repo_resource.url, token);
    repo_resource.url = authRepoUrl;
  }

  const { repo_name, safeDirectoryPath: cloneSafeDirectoryPath } =
    await git_clone(
      cwd,
      repo_resource,
      use_individual_branch || workspace_id.startsWith(FORKED_WORKSPACE_PREFIX),
    );
  safeDirectoryPath = cloneSafeDirectoryPath;

  // GPG signing must be configured (local commit.gpgsign / user.signingkey /
  // gpg.program wrapper) BEFORE the CLI commits. The committer email/name
  // are passed through to the CLI so authorship stays identical to the old
  // in-script git_push (committer email = gpg key email when signing).
  if (repo_resource.gpg_key) {
    await set_gpg_signing_secret(repo_resource.gpg_key);
  }

  const subfolder = repo_resource.folder ?? "";
  const branch_or_default = repo_resource.branch ?? "<DEFAULT>";
  console.log(
    `Pushing to repository ${repo_name} in subfolder ${subfolder} on branch ${branch_or_default}`,
  );

  try {
    // Raw flags are passed through verbatim; the CLI replicates the
    // fork-disable, parent-of-fork rooting and include/promotion derivation.
    const args = [
      "sync",
      "git-deploy",
      "--token",
      process.env["WM_TOKEN"] ?? "",
      "--workspace",
      workspace_id,
      "--base-url",
      process.env["BASE_URL"] + "/",
      "--repository",
      repo_url_resource_path,
      "--git-deploy-items",
      JSON.stringify(items),
    ];
    if (use_individual_branch) args.push("--use-individual-branch");
    if (group_by_folder) args.push("--group-by-folder");
    if (only_create_branch) args.push("--only-create-branch");
    if (parent_workspace_id) {
      args.push("--parent-workspace-id", parent_workspace_id);
    }
    if (skip_secret) args.push("--skip-secrets");
    if (repo_resource.gpg_key) {
      args.push("--git-committer-email", repo_resource.gpg_key.email);
      args.push("--git-committer-name", process.env["WM_USERNAME"] ?? "");
    }
    await wmill_run(3, ...args);
  } catch (e) {
    throw e;
  } finally {
    await delete_pgp_keys();
    // Cleanup: remove safe.directory config
    if (safeDirectoryPath) {
      try {
        await sh_run(
          undefined,
          "git",
          "config",
          "--global",
          "--unset",
          "safe.directory",
          safeDirectoryPath,
        );
      } catch (e) {
        console.log(`Warning: Could not unset safe.directory config: ${e}`);
      }
    }
  }
  console.log("Finished syncing");
  process.chdir(`${cwd}`);
}

async function git_clone(
  cwd: string,
  repo_resource: any,
  no_single_branch: boolean,
): Promise<{
  repo_name: string;
  safeDirectoryPath: string;
  clonedBranchName: string;
}> {
  // TODO: handle private SSH keys as well
  let repo_url = repo_resource.url;
  const subfolder = repo_resource.folder ?? "";
  const branch = repo_resource.branch ?? "";
  const repo_name = basename(repo_url, ".git");
  const azureMatch = repo_url.match(/AZURE_DEVOPS_TOKEN\((?<url>.+)\)/);
  if (azureMatch) {
    console.log(
      "Requires Azure DevOps service account access token, requesting...",
    );
    const azureResource = await wmillclient.getResource(azureMatch.groups.url);
    const response = await fetch(
      `https://login.microsoftonline.com/${azureResource.azureTenantId}/oauth2/token`,
      {
        method: "POST",
        body: new URLSearchParams({
          client_id: azureResource.azureClientId,
          client_secret: azureResource.azureClientSecret,
          grant_type: "client_credentials",
          resource: "499b84ac-1321-427f-aa17-267ca6975798/.default",
        }),
      },
    );
    const { access_token } = await response.json();
    repo_url = repo_url.replace(azureMatch[0], access_token);
  }
  const args = ["clone", "--quiet", "--depth", "1"];
  if (no_single_branch) {
    args.push("--no-single-branch"); // needed in case the asset branch already exists in the repo
  }
  if (subfolder !== "") {
    args.push("--sparse");
  }
  if (branch !== "") {
    args.push("--branch");
    args.push(branch);
  }
  args.push(repo_url);
  args.push(repo_name);
  await sh_run(-1, "git", ...args);
  try {
    process.chdir(`${cwd}/${repo_name}`);
    const safeDirectoryPath = process.cwd();
    // Add safe.directory to handle dubious ownership in cloned repo
    try {
      await sh_run(
        undefined,
        "git",
        "config",
        "--global",
        "--add",
        "safe.directory",
        process.cwd(),
      );
    } catch (e) {
      console.log(`Warning: Could not add safe.directory config: ${e}`);
    }

    if (subfolder !== "") {
      await sh_run(undefined, "git", "sparse-checkout", "add", subfolder);
      try {
        process.chdir(`${cwd}/${repo_name}/${subfolder}`);
      } catch (err) {
        console.log(
          `Error changing directory to '${cwd}/${repo_name}/${subfolder}'. Error was:\n${err}`,
        );
        throw err;
      }
    }
    const clonedBranchName = (
      await sh_run(undefined, "git", "rev-parse", "--abbrev-ref", "HEAD")
    ).trim();
    return { repo_name, safeDirectoryPath, clonedBranchName };
  } catch (err) {
    console.log(
      `Error changing directory to '${cwd}/${repo_name}'. Error was:\n${err}`,
    );
    throw err;
  }
}

async function sh_run(
  secret_position: number | undefined,
  cmd: string,
  ...args: string[]
) {
  const nargs = secret_position != undefined ? args.slice() : args;
  if (secret_position && secret_position < 0) {
    secret_position = nargs.length - 1 + secret_position;
  }
  let secret: string | undefined = undefined;
  if (secret_position != undefined) {
    nargs[secret_position] = "***";
    secret = args[secret_position];
  }

  console.log(`Running '${cmd} ${nargs.join(" ")} ...'`);
  const command = exec(`${cmd} ${args.join(" ")}`);
  try {
    const { stdout, stderr } = await command;
    if (stdout.length > 0) {
      console.log(stdout);
    }
    if (stderr.length > 0) {
      console.log(stderr);
    }
    console.log("Command successfully executed");
    return stdout;
  } catch (error) {
    let errorString = error.toString();
    if (secret) {
      errorString = errorString.replace(secret, "***");
    }
    const err = `SH command '${cmd} ${nargs.join(
      " ",
    )}' returned with error ${errorString}`;
    throw Error(err);
  }
}

async function wmill_run(secret_position: number, ...cmd: string[]) {
  cmd = cmd.filter((elt) => elt !== "");
  const cmd2 = cmd.slice();
  cmd2[secret_position] = "***";
  console.log(`Running 'wmill ${cmd2.join(" ")} ...'`);
  await wmill.parse(cmd);
  console.log("Command successfully executed");
}

// Import the key, write a stateless gpg wrapper, and point git at it.
//
// WIN-1974 fix: the previous version pre-warmed gpg-agent's passphrase cache
// with a dummy `gpg -bsau` immediately before handing control to the CLI.
// That cache was no longer reliably valid by the time the CLI's `git commit`
// ran (workspace API resolution, zip pull, file extract, lockfile autofill
// all run in between), so signing failed with `gpg failed to sign the data`.
// Replacing the agent-cache dependency with a `gpg.program` wrapper that
// always uses `--pinentry-mode loopback` (and a `--passphrase-file` when a
// passphrase exists) makes signing stateless: every git-invoked gpg call
// provides the passphrase itself instead of trusting the agent.
async function set_gpg_signing_secret(gpg_key: GpgKey) {
  try {
    console.log("Setting GPG private key for git commits");

    const formattedGpgContent = gpg_key.private_key.replace(
      /(-----BEGIN PGP PRIVATE KEY BLOCK-----)([\s\S]*?)(-----END PGP PRIVATE KEY BLOCK-----)/,
      (_: string, header: string, body: string, footer: string) =>
        header +
        "\n" +
        "\n" +
        body.replace(/ ([^\s])/g, "\n$1").trim() +
        "\n" +
        footer,
    );

    await sh_run(undefined, "mkdir", "-p", GPG_HOME);
    await sh_run(undefined, "chmod", "700", GPG_HOME);
    process.env.GNUPGHOME = GPG_HOME;

    try {
      await sh_run(
        1,
        "bash",
        "-c",
        `cat <<EOF | gpg --batch --import \n${formattedGpgContent}\nEOF`,
      );
    } catch (e) {
      // Original error would contain sensitive data
      throw new Error("Failed to import GPG key!");
    }

    const listKeysOutput = await sh_run(
      undefined,
      "gpg",
      "--list-secret-keys",
      "--with-colons",
      "--keyid-format=long",
    );

    const keyInfoMatch = listKeysOutput.match(
      /sec:[^:]*:[^:]*:[^:]*:([A-F0-9]+):.*\nfpr:::::::::([A-F0-9]{40}):/,
    );

    if (!keyInfoMatch) {
      throw new Error("Failed to extract GPG Key ID and Fingerprint");
    }

    const keyId = keyInfoMatch[1];
    gpgFingerprint = keyInfoMatch[2];

    // Write a stateless gpg wrapper: every sign goes through
    // `--pinentry-mode loopback` (+ --passphrase-file when set), so signing
    // never depends on gpg-agent having a cached passphrase. Passphrase file
    // is mode-600 via umask 077; both files are removed in delete_pgp_keys.
    if (gpg_key.passphrase) {
      // Escape single quotes for shell-safe single-quoting:  '  ->  '\''
      const escapedPassphrase = gpg_key.passphrase.replace(/'/g, "'\\''");
      await sh_run(
        2, // mask the passphrase arg in logs
        "bash",
        "-c",
        `umask 077 && printf %s '${escapedPassphrase}' > ${GPG_PASSPHRASE_FILE}`,
      );
    }
    const wrapperBody = gpg_key.passphrase
      ? `#!/bin/sh\nexec gpg --pinentry-mode loopback --passphrase-file "${GPG_PASSPHRASE_FILE}" --batch "$@"\n`
      : `#!/bin/sh\nexec gpg --pinentry-mode loopback --batch "$@"\n`;
    await sh_run(
      undefined,
      "bash",
      "-c",
      `cat > ${GPG_WRAPPER} <<'WRAPPER_EOF' && chmod 700 ${GPG_WRAPPER}\n${wrapperBody}WRAPPER_EOF`,
    );
    gpgWrapperWritten = true;

    // Configure Git: use the extracted key, sign every commit, and route
    // every gpg invocation through the wrapper. All three are LOCAL repo
    // config, not --global — so two concurrent deploys against different
    // repos don't trample each other.
    await sh_run(undefined, "git", "config", "user.signingkey", keyId);
    await sh_run(undefined, "git", "config", "commit.gpgsign", "true");
    await sh_run(undefined, "git", "config", "gpg.program", GPG_WRAPPER);
    console.log(`GPG signing configured with key ID: ${keyId}`);
  } catch (e) {
    console.error(`Failure while setting GPG key: ${e} `);
    await delete_pgp_keys();
  }
}

async function delete_pgp_keys() {
  console.log("deleting gpg keys");
  if (gpgFingerprint) {
    await sh_run(
      undefined,
      "gpg",
      "--batch",
      "--yes",
      "--pinentry-mode",
      "loopback",
      "--delete-secret-key",
      gpgFingerprint,
    );
    await sh_run(
      undefined,
      "gpg",
      "--batch",
      "--yes",
      "--delete-key",
      "--pinentry-mode",
      "loopback",
      gpgFingerprint,
    );
  }
  if (gpgWrapperWritten) {
    try {
      await sh_run(undefined, "rm", "-f", GPG_PASSPHRASE_FILE, GPG_WRAPPER);
    } catch (e) {
      console.log(`Warning: Could not remove gpg wrapper artifacts: ${e}`);
    }
  }
}

async function get_gh_app_token() {
  const workspace = process.env["WM_WORKSPACE"];
  const jobToken = process.env["WM_TOKEN"];

  const baseUrl =
    process.env["BASE_INTERNAL_URL"] ??
    process.env["BASE_URL"] ??
    "http://localhost:8000";

  const url = `${baseUrl}/api/w/${workspace}/github_app/token`;

  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${jobToken}`,
    },
    body: JSON.stringify({
      job_token: jobToken,
    }),
  });

  if (!response.ok) {
    const errorBody = await response.text().catch(() => "");
    throw new Error(
      `GitHub App token error (${response.status}): ${errorBody || response.statusText}`,
    );
  }

  const data = await response.json();

  return data.token;
}

function prependTokenToGitHubUrl(gitHubUrl: string, installationToken: string) {
  if (!gitHubUrl || !installationToken) {
    throw new Error("Both GitHub URL and Installation Token are required.");
  }

  const url = new URL(gitHubUrl);
  return `https://x-access-token:${installationToken}@${url.hostname}${url.pathname}`;
}
