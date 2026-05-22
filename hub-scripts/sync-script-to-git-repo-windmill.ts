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
const FORKED_BRANCH_PREFIX = "wm-fork";

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
    items = [{
      path_type,
      path,
      parent_path,
      commit_msg,
    }];
  }
  await inner(items, workspace_id, repo_url_resource_path, skip_secret, use_individual_branch, group_by_folder, only_create_branch, parent_workspace_id);
}

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
        `Syncing ${item.path_type} ${item.path ?? ""} with parent ${item.parent_path ?? ""}`
      );
    }
  }

  if (repo_resource.is_github_app) {
    const token = await get_gh_app_token();
    const authRepoUrl = prependTokenToGitHubUrl(repo_resource.url, token);
    repo_resource.url = authRepoUrl;
  }

  const { repo_name, safeDirectoryPath: cloneSafeDirectoryPath, clonedBranchName } = await git_clone(cwd, repo_resource, use_individual_branch || workspace_id.startsWith(FORKED_WORKSPACE_PREFIX));
  safeDirectoryPath = cloneSafeDirectoryPath;


  // Since we don't modify the resource on the forked workspaces, we have to cosnider the case of
  // a fork of a fork workspace. In that case, the original branch is not stored in the resource
  // settings, but we need to infer it from the workspace id

  if (workspace_id.startsWith(FORKED_WORKSPACE_PREFIX)) {
    if (use_individual_branch) {
      console.log("Cannot have `use_individual_branch` in a forked workspace, disabling option`");
      use_individual_branch = false;
    }
    if (group_by_folder) {
      console.log("Cannot have `group_by_folder` in a forked workspace, disabling option`");
      group_by_folder = false;
    }
  }

  if (parent_workspace_id && parent_workspace_id.startsWith(FORKED_WORKSPACE_PREFIX)) {
    const parentBranch = get_fork_branch_name(parent_workspace_id, clonedBranchName);
    console.log(`This workspace's parent is also a fork, moving to branch ${parentBranch} in case a new branch needs to be created with the appropriate root`);
    await git_checkout_branch(
      items,
      parent_workspace_id,
      use_individual_branch,
      group_by_folder,
      clonedBranchName
    );
  }

  await git_checkout_branch(
    items,
    workspace_id,
    use_individual_branch,
    group_by_folder,
    clonedBranchName
  );


  const subfolder = repo_resource.folder ?? "";
  const branch_or_default = repo_resource.branch ?? "<DEFAULT>";
  console.log(
    `Pushing to repository ${repo_name} in subfolder ${subfolder} on branch ${branch_or_default}`
  );

  // If we want to just create the branch, we can skip pulling the changes.
  if (!only_create_branch) {
    await wmill_sync_pull(
      items,
      workspace_id,
      skip_secret,
      repo_url_resource_path,
      use_individual_branch,
      repo_resource.branch
    );
  }
  try {
    await git_push(items, repo_resource, only_create_branch);
  } catch (e) {
    throw e;
  } finally {
    await delete_pgp_keys();
    // Cleanup: remove safe.directory config
    if (safeDirectoryPath) {
      try {
        await sh_run(undefined, "git", "config", "--global", "--unset", "safe.directory", safeDirectoryPath);
      } catch (e) {
        console.log(`Warning: Could not unset safe.directory config: ${e}`);
      }
    }
  }
  console.log("Finished syncing");
  process.chdir(`${cwd}`);
}


function get_fork_branch_name(w_id: string, originalBranch: string): string {
  if (w_id.startsWith(FORKED_WORKSPACE_PREFIX)) {
    return w_id.replace(FORKED_WORKSPACE_PREFIX, `${FORKED_BRANCH_PREFIX}/${originalBranch}/`);
  }
  return w_id;
}

async function git_clone(
  cwd: string,
  repo_resource: any,
  no_single_branch: boolean,
): Promise<{ repo_name: string; safeDirectoryPath: string; clonedBranchName: string }> {
  // TODO: handle private SSH keys as well
  let repo_url = repo_resource.url;
  const subfolder = repo_resource.folder ?? "";
  const branch = repo_resource.branch ?? "";
  const repo_name = basename(repo_url, ".git");
  const azureMatch = repo_url.match(/AZURE_DEVOPS_TOKEN\((?<url>.+)\)/);
  if (azureMatch) {
    console.log(
      "Requires Azure DevOps service account access token, requesting..."
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
      }
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
      await sh_run(undefined, "git", "config", "--global", "--add", "safe.directory", process.cwd());
    } catch (e) {
      console.log(`Warning: Could not add safe.directory config: ${e}`);
    }

    if (subfolder !== "") {
      await sh_run(undefined, "git", "sparse-checkout", "add", subfolder);
      try {
        process.chdir(`${cwd}/${repo_name}/${subfolder}`);
      } catch (err) {
        console.log(
          `Error changing directory to '${cwd}/${repo_name}/${subfolder}'. Error was:\n${err}`
        );
        throw err;
      }
    }
    const clonedBranchName = (await sh_run(undefined, "git", "rev-parse", "--abbrev-ref", "HEAD")).trim();
    return { repo_name, safeDirectoryPath, clonedBranchName };

  } catch (err) {
    console.log(
      `Error changing directory to '${cwd}/${repo_name}'. Error was:\n${err}`
    );
    throw err;
  }
}
async function git_checkout_branch(
  items: SyncObject[],
  workspace_id: string,
  use_individual_branch: boolean,
  group_by_folder: boolean,
  originalBranchName: string
) {
  let branchName;
  if (workspace_id.startsWith(FORKED_WORKSPACE_PREFIX)) {
    branchName = get_fork_branch_name(workspace_id, originalBranchName);
  } else {

    if (!use_individual_branch
      // If individual branch is true, we can assume items is of length 1, as debouncing is disabled for jobs with this flag
      || items[0].path_type === "user" || items[0].path_type === "group") {
      return;
    }

    // as mentioned above, it is safe to assume that items.len is 1
    const [path, parent_path] = [items[0].path, items[0].parent_path];
    branchName = group_by_folder
      ? `wm_deploy/${workspace_id}/${(path ?? parent_path)
        ?.split("/")
        .slice(0, 2)
        .join("__")}`
      : `wm_deploy/${workspace_id}/${items[0].path_type}/${(
        path ?? parent_path
      )?.replaceAll("/", "__")}`;
  }

  try {
    await sh_run(undefined, "git", "checkout", branchName);
  } catch (err) {
    console.log(
      `Error checking out branch ${branchName}. It is possible it doesn't exist yet, tentatively creating it... Error was:\n${err}`
    );
    try {
      await sh_run(undefined, "git", "checkout", "-b", branchName);
      await sh_run(
        undefined,
        "git",
        "config",
        "--add",
        "--bool",
        "push.autoSetupRemote",
        "true"
      );
    } catch (err) {
      console.log(
        `Error checking out branch '${branchName}'. Error was:\n${err}`
      );
      throw err;
    }
  }
  console.log(`Successfully switched to branch ${branchName}`);
}

function composeCommitHeader(items: SyncObject[]): string {
  // Count occurrences of each path_type
  const typeCounts = new Map<PathType, number>();
  for (const item of items) {
    typeCounts.set(item.path_type, (typeCounts.get(item.path_type) ?? 0) + 1);
  }

  // Sort by count descending to get the top 2
  const sortedTypes = Array.from(typeCounts.entries()).sort((a, b) => b[1] - a[1]);

  const parts: string[] = [];
  let othersCount = 0;

  for (let i = 0; i < sortedTypes.length; i++) {
    const [pathType, count] = sortedTypes[i];
    if (i < 3) {
      // Pluralize the path type if count > 1
      const label = count > 1 ? `${pathType}s` : pathType;

      if (i == 2 && sortedTypes.length == 3) {
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

async function git_push(
  items: SyncObject[],
  repo_resource: any,
  only_create_branch: boolean,
) {
  let user_email = process.env["WM_EMAIL"] ?? "";
  let user_name = process.env["WM_USERNAME"] ?? "";

  if (repo_resource.gpg_key) {
    await set_gpg_signing_secret(repo_resource.gpg_key);
    // Configure git with GPG key email for signing
    await sh_run(
      undefined,
      "git",
      "config",
      "user.email",
      repo_resource.gpg_key.email
    );
    await sh_run(undefined, "git", "config", "user.name", user_name);
  } else {
    await sh_run(undefined, "git", "config", "user.email", user_email);
    await sh_run(undefined, "git", "config", "user.name", user_name);
  }
  if (only_create_branch) {
    await sh_run(undefined, "git", "push", "--porcelain");
  }

  let commit_description: string[] = [];
  for (const { path, parent_path, commit_msg } of items) {
    if (path !== undefined && path !== null && path !== "") {
      try {
        await sh_run(undefined, "git", "add", "wmill-lock.yaml", `'${path}**'`);
      } catch (e) {
        console.log(`Unable to stage files matching ${path}**, ${e}`);
      }
    }
    if (parent_path !== undefined && parent_path !== null && parent_path !== "") {
      try {
        await sh_run(
          undefined,
          "git",
          "add",
          "wmill-lock.yaml",
          `'${parent_path}**'`
        );
      } catch (e) {
        console.log(`Unable to stage files matching ${parent_path}, ${e}`);
      }
    }

    commit_description.push(commit_msg);
  }

  try {
    await sh_run(undefined, "git", "diff", "--cached", "--quiet");
  } catch {
    // git diff returns exit-code = 1 when there's at least one staged changes
    const commitArgs = ["git", "commit"];

    // Always use --author to set consistent authorship
    commitArgs.push("--author", `"${user_name} <${user_email}>"`);

    const [header, description] = (commit_description.length == 1)
      ? [commit_description[0], ""]
      : [composeCommitHeader(items), commit_description.join("\n")];

    commitArgs.push(
      "-m",
      `"${header == undefined || header == "" ? "no commit msg" : header}"`,
      "-m",
      `"${description}"`
    );

    await sh_run(undefined, ...commitArgs);
    try {
      await sh_run(undefined, "git", "push", "--porcelain");
    } catch (e) {
      console.log(`Could not push, trying to rebase first: ${e}`);
      await sh_run(undefined, "git", "pull", "--rebase");
      await sh_run(undefined, "git", "push", "--porcelain");
    }
    return;
  }

  console.log("No changes detected, nothing to commit. Returning...");
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
  // new Deno.Command(cmd, {
  //   args: args,
  // });
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
      " "
    )}' returned with error ${errorString}`;
    throw Error(err);
  }
}

function regexFromPath(path_type: PathType, path: string) {
  if (path_type == "flow") {
    return `${path}.flow/*,${path}__flow/*`;
  } else if (path_type == "app") {
    return `${path}.app/*,${path}__app/*`;
  } else if (path_type == "raw_app") {
    return `${path}.raw_app/**,${path}__raw_app/**`;
  } else if (path_type == "folder") {
    return `${path}/folder.meta.*`;
  } else if (path_type == "resourcetype") {
    return `${path}.resource-type.*`;
  } else if (path_type == "resource") {
    return `${path}.resource.*`;
  } else if (path_type == "variable") {
    return `${path}.variable.*`;
  } else if (path_type == "schedule") {
    return `${path}.schedule.*`;
  } else if (path_type == "user") {
    return `${path}.user.*`;
  } else if (path_type == "group") {
    return `${path}.group.*`;
  } else if (path_type == "httptrigger") {
    return `${path}.http_trigger.*`;
  } else if (path_type == "websockettrigger") {
    return `${path}.websocket_trigger.*`;
  } else if (path_type == "kafkatrigger") {
    return `${path}.kafka_trigger.*`;
  } else if (path_type == "natstrigger") {
    return `${path}.nats_trigger.*`;
  } else if (path_type == "postgrestrigger") {
    return `${path}.postgres_trigger.*`;
  } else if (path_type == "mqtttrigger") {
    return `${path}.mqtt_trigger.*`;
  } else if (path_type == "sqstrigger") {
    return `${path}.sqs_trigger.*`;
  } else if (path_type == "gcptrigger") {
    return `${path}.gcp_trigger.*`;
  } else if (path_type == "azuretrigger") {
    return `${path}.azure_trigger.*`;
  } else if (path_type == "emailtrigger") {
    return `${path}.email_trigger.*`;
  } else {
    return `${path}.*`;
  }
}

async function wmill_sync_pull(
  items: SyncObject[],
  workspace_id: string,
  skip_secret: boolean,
  repo_url_resource_path: string,
  use_individual_branch: boolean,
  original_branch?: string,
) {
  const includes = [];
  for (const item of items) {
    const { path_type, path, parent_path } = item;
    if (path !== undefined && path !== null && path !== "") {
      includes.push(regexFromPath(path_type, path));
    }
    if (parent_path !== undefined && parent_path !== null && parent_path !== "") {
      includes.push(regexFromPath(path_type, parent_path));
    }
  }

  console.log("Pulling workspace into git repo");

  const args = [
    "sync",
    "pull",
    "--token",
    process.env["WM_TOKEN"] ?? "",
    "--workspace",
    workspace_id,
    "--base-url",
    process.env["BASE_URL"] + "/",
    "--repository",
    repo_url_resource_path,
    "--yes",
    skip_secret ? "--skip-secrets" : "",
  ];

  if (items.some(item => item.path_type === "schedule") && !use_individual_branch) {
    args.push("--include-schedules");
  }

  if (items.some(item => item.path_type === "group") && !use_individual_branch) {
    args.push("--include-groups");
  }

  if (items.some(item => item.path_type === "user") && !use_individual_branch) {
    args.push("--include-users");
  }

  if (items.some(item => item.path_type.includes("trigger")) && !use_individual_branch) {
    args.push("--include-triggers");
  }
  // Only include settings when specifically deploying settings
  if (items.some(item => item.path_type === "settings") && !use_individual_branch) {
    args.push("--include-settings");
  }

  // Only include key when specifically deploying keys
  if (items.some(item => item.path_type === "key") && !use_individual_branch) {
    args.push("--include-key");
  }

  args.push("--extra-includes", includes.join(","));

  // If using individual branches, apply promotion settings from original branch
  if (use_individual_branch && original_branch) {
    console.log(`Individual branch deployment detected - using promotion settings from '${original_branch}'`);
    args.push("--promotion", original_branch);
  }

  await wmill_run(3, ...args);
}

async function wmill_run(secret_position: number, ...cmd: string[]) {
  cmd = cmd.filter((elt) => elt !== "");
  const cmd2 = cmd.slice();
  cmd2[secret_position] = "***";
  console.log(`Running 'wmill ${cmd2.join(" ")} ...'`);
  await wmill.parse(cmd);
  console.log("Command successfully executed");
}

// Function to set up GPG signing
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
        footer
    );

    const gpg_path = `/tmp/gpg`;
    await sh_run(undefined, "mkdir", "-p", gpg_path);
    await sh_run(undefined, "chmod", "700", gpg_path);
    process.env.GNUPGHOME = gpg_path;
    // process.env.GIT_TRACE = 1;

    try {
      await sh_run(
        1,
        "bash",
        "-c",
        `cat <<EOF | gpg --batch --import \n${formattedGpgContent}\nEOF`
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
      "--keyid-format=long"
    );

    const keyInfoMatch = listKeysOutput.match(
      /sec:[^:]*:[^:]*:[^:]*:([A-F0-9]+):.*\nfpr:::::::::([A-F0-9]{40}):/
    );

    if (!keyInfoMatch) {
      throw new Error("Failed to extract GPG Key ID and Fingerprint");
    }

    const keyId = keyInfoMatch[1];
    gpgFingerprint = keyInfoMatch[2];

    if (gpg_key.passphrase) {
      // This is adummy command to unlock the key
      // with passphrase to load it into agent
      await sh_run(
        1,
        "bash",
        "-c",
        `echo "dummy" | gpg --batch --pinentry-mode loopback --passphrase '${gpg_key.passphrase}' --status-fd=2 -bsau ${keyId}`
      );
    }

    // Configure Git to use the extracted key
    await sh_run(undefined, "git", "config", "user.signingkey", keyId);
    await sh_run(undefined, "git", "config", "commit.gpgsign", "true");
    console.log(`GPG signing configured with key ID: ${keyId} `);
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
      gpgFingerprint
    );
    await sh_run(
      undefined,
      "gpg",
      "--batch",
      "--yes",
      "--delete-key",
      "--pinentry-mode",
      "loopback",
      gpgFingerprint
    );
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
    throw new Error(`GitHub App token error (${response.status}): ${errorBody || response.statusText}`);
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

