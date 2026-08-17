import * as wmillclient from "windmill-client";
import { basename, join } from "node:path";
import { existsSync, rmSync } from "fs";
import process from "process";
import { spawn } from 'child_process';
import * as fs_async from 'fs/promises';
import * as fs from 'node:fs';

const UPLOAD_CONCURRENCY = 16;
const CLONE_MARKER_FILE = ".windmill_clone_complete";

type GitRepository = {
  url: string;
  branch: string;
  folder: string;
  gpg_key: any;
  is_github_app: boolean;
};

export async function main(
  resource_path: string,
  workspace: string,
  git_ssh_identity?: string[],
  commit?: string
) {
  let clonedRepoPath: string | undefined;

  try {
    console.log("Starting git clone and Blob storage upload process");

    // Get the git repository resource
    const repo_resource: GitRepository = await wmillclient.getResource(resource_path);

    const cwd = process.cwd();

    if (git_ssh_identity) {
      process.env.GIT_SSH_COMMAND = await get_git_ssh_cmd(cwd, git_ssh_identity)
    }

    // Handle GitHub App authentication if needed
    if (repo_resource.is_github_app) {
      const token = await get_gh_app_token();
      repo_resource.url = prependTokenToGitHubUrl(repo_resource.url, token);
    }

    process.env["HOME"] = ".";
    process.env.GIT_TERMINAL_PROMPT = "0";

    // Clone the repository
    const { repo_name, commitHash } = await git_clone(cwd, repo_resource, commit);
    clonedRepoPath = join(cwd, repo_name);

    // Remove .git directory to avoid uploading git history
    const gitDir = join(clonedRepoPath, ".git");
    if (existsSync(gitDir)) {
      rmSync(gitDir, { recursive: true, force: true });
      console.log("Removed .git directory");
    }

    // Upload to S3
    const s3Path = `gitrepos/${workspace}/${resource_path}/${commitHash}`;
    const fileCount = await uploadDirectoryToS3(clonedRepoPath, s3Path, workspace);

    return {
      success: true,
      message: "Repository cloned and uploaded to S3 successfully",
      s3_path: s3Path,
      commit_hash: commitHash,
      file_count: fileCount,
    };

  } catch (error) {
    console.error("Error in git clone and upload:", error);
    throw error;
  } finally {
    // Clean up cloned repository
    if (clonedRepoPath && existsSync(clonedRepoPath)) {
      rmSync(clonedRepoPath, { recursive: true, force: true });
      console.log("Cleaned up cloned repository");
    }
  }
}

async function get_git_ssh_cmd(cwd: string, git_ssh_identity: string[]): Promise<string> {
  const sshIdFiles = await Promise.all(
    git_ssh_identity.map(async (varPath, i) => {
      const filePath = join(cwd, `./ssh_id_priv_${i}`);

      try {
        // Get variable value using windmill
        let content = await wmillclient.getVariable(varPath);
        content += '\n';

        // Write file with content
        await fs_async.writeFile(filePath, content, { encoding: 'utf8' });

        // Set file permissions to 0o600 (read/write for owner only)
        await fs_async.chmod(filePath, 0o600);

        // Escape single quotes for shell command
        const escapedPath = filePath.replace(/'/g, "'\\''");
        return ` -i '${escapedPath}'`;
      } catch (error) {
        console.error(
          `Variable ${varPath} not found for git ssh identity: ${error}`
        );
        return '';
      }
    })
  );

  const gitSshCmd = `ssh -o StrictHostKeyChecking=no${sshIdFiles.join('')}`;
  return gitSshCmd;
}

async function git_clone(
  cwd: string,
  repo_resource: GitRepository,
  commit?: string,
): Promise<{ repo_name: string; commitHash: string }> {
    if (commit) {
      return git_clone_at_commit(cwd, repo_resource, commit);
    } else {
      return git_clone_at_latest(cwd, repo_resource);
    }
}

async function git_clone_at_commit(
  cwd: string,
  repo_resource: GitRepository,
  commit: string,
): Promise<{ repo_name: string; commitHash: string }> {
  let repo_url = repo_resource.url;
  const subfolder = repo_resource.folder ?? "";
  let branch = repo_resource.branch ?? "";
  const repo_name = basename(repo_url, ".git");

  const azureMatch = repo_url.match(/AZURE_DEVOPS_TOKEN\((?<url>.+)\)/);
  if (azureMatch) {
    console.log("Fetching Azure DevOps access token...");
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

  const repoPath = join(cwd, repo_name);
  await fs_async.mkdir(repoPath, { recursive: true });

  process.chdir(repoPath);

  let args = ['init', '--quiet']
  if (branch) {
    args.push(`--initial-branch=${branch}`)
  }
  await runCommand(undefined, 'git', ...args);

  await runCommand(0, 'git', 'remote', 'add', 'origin', repo_url);

  await runCommand(undefined, 'git', 'fetch', '--depth=1', '--quiet', 'origin', commit);

  await runCommand(undefined, 'git', 'checkout', '--quiet', 'FETCH_HEAD');

  const commitHash = (await runCommand(undefined, "git", "rev-parse", "HEAD")).trim();

  // Return to original directory
  process.chdir(cwd);

  return { repo_name, commitHash };
}

async function git_clone_at_latest(
  cwd: string,
  repo_resource: GitRepository
): Promise<{ repo_name: string; commitHash: string }> {
  let repo_url = repo_resource.url;
  const subfolder = repo_resource.folder ?? "";
  let branch = repo_resource.branch ?? "";
  const repo_name = basename(repo_url, ".git");

  // Handle Azure DevOps token if needed
  const azureMatch = repo_url.match(/AZURE_DEVOPS_TOKEN\((?<url>.+)\)/);
  if (azureMatch) {
    console.log("Fetching Azure DevOps access token...");
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
  if (subfolder !== "") args.push("--sparse");
  if (branch !== "") args.push("--branch", branch);
  args.push(repo_url, repo_name);

  await runCommand(-1, "git", ...args);

  const fullPath = join(cwd, repo_name);
  process.chdir(fullPath);

  if (subfolder !== "") {
    await runCommand(undefined, "git", "sparse-checkout", "add", subfolder);
    const subfolderPath = join(fullPath, subfolder);

    if (!existsSync(subfolderPath)) {
      throw new Error(`Subfolder ${subfolder} does not exist.`);
    }

    process.chdir(subfolderPath);
  }

  // Get the commit hash
  const commitHash = (await runCommand(undefined, "git", "rev-parse", "HEAD")).trim();

  // Return to original directory
  process.chdir(cwd);

  return { repo_name, commitHash };
}

async function uploadDirectoryToS3(
  directoryPath: string,
  s3BasePath: string,
  workspace: string,
): Promise<number> {
  console.log(`Uploading ${directoryPath} -> ${s3BasePath}`);

  // Walk once into a flat task list so we can drive a bounded-concurrency pool.
  const tasks: { localPath: string; s3Key: string }[] = [];
  function walk(dir: string, s3Path: string) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const fullPath = join(dir, entry.name);
      const s3Key = s3Path ? `${s3Path}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        walk(fullPath, s3Key);
      } else if (entry.isFile()) {
        tasks.push({ localPath: fullPath, s3Key });
      }
    }
  }
  walk(directoryPath, s3BasePath);
  console.log(`Discovered ${tasks.length} files to upload`);

  let nextIndex = 0;
  let uploaded = 0;
  let lastReport = 0;
  async function worker() {
    while (true) {
      const idx = nextIndex++;
      if (idx >= tasks.length) return;
      const { localPath, s3Key } = tasks[idx];
      const fileContent = fs.readFileSync(localPath);
      const blob = new Blob([fileContent], { type: 'application/octet-stream' });
      await wmillclient.HelpersService.gitRepoViewerFileUpload({
        workspace,
        fileKey: s3Key,
        requestBody: blob,
      });
      uploaded++;
      if (uploaded - lastReport >= 25 || uploaded === tasks.length) {
        lastReport = uploaded;
        console.log(`Uploaded ${uploaded} / ${tasks.length} files`);
      }
    }
  }
  await Promise.all(
    Array.from({ length: Math.min(UPLOAD_CONCURRENCY, tasks.length) }, () => worker())
  );

  // Marker is the LAST write — its presence is what the viewer checks for.
  const markerKey = `${s3BasePath}/${CLONE_MARKER_FILE}`;
  const markerBody = JSON.stringify({
    completed_at: new Date().toISOString(),
    file_count: tasks.length,
  });
  await wmillclient.HelpersService.gitRepoViewerFileUpload({
    workspace,
    fileKey: markerKey,
    requestBody: new Blob([markerBody], { type: 'application/json' }),
  });
  console.log(`Wrote completion marker: ${markerKey}`);

  return tasks.length;
}

function runCommand(secret_position: number | undefined, cmd: string, ...args: string[]): Promise<string> {
  const nargs = secret_position != undefined ? args.slice() : args;
  if (secret_position && secret_position < 0)
    secret_position = nargs.length - 1 + secret_position;

  let secret: string | undefined = undefined;
  if (secret_position != undefined) {
    nargs[secret_position] = "***";
    secret = args[secret_position];
  }
  console.log(`Running shell command: '${cmd} ${nargs.join(" ")} ...'`);

  return new Promise((resolve, reject) => {
    const process = spawn(cmd, args);

    let stdout = '';
    let stderr = '';

    process.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    process.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    process.on('error', (error) => {
      let errorString = error.toString();
      if (secret) errorString = errorString.replace(secret, "***");
      console.log(`Shell command FAILED: ${cmd}`, errorString);
      const e = new Error(
        `SH command '${cmd} ${nargs.join(" ")}' failed: ${errorString}`
      );
      reject(e);
    });

    process.on('close', (code) => {
      if (stdout.length > 0) {
        console.log("Shell stdout:", stdout);
      }
      if (stderr.length > 0) {
        console.log("Shell stderr:", stderr);
      }
      if (code === 0) {
        console.log(`Shell command completed successfully: ${cmd}`);
        resolve(stdout);
      } else {
        reject(new Error(`Command failed with code ${code}: ${stderr}`));
      }
    });
  });
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
    body: JSON.stringify({ job_token: jobToken }),
  });

  if (!response.ok) {
    const errorBody = await response.text().catch(() => "");
    throw new Error(`GitHub App token error (${response.status}): ${errorBody || response.statusText}`);
  }
  const data = await response.json();
  return data.token;
}

function prependTokenToGitHubUrl(gitHubUrl: string, installationToken: string) {
  const url = new URL(gitHubUrl);
  return `https://x-access-token:${installationToken}@${url.hostname}${url.pathname}`;
}
