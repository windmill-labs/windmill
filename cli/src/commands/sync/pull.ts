import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import JSZip from "jszip";
import { extract } from "tar-stream";
import { Readable } from "node:stream";
import { Workspace } from "../workspace/workspace.ts";
import { getHeaders } from "../../utils/utils.ts";

/**
 * Adapter that wraps tar entries in a JSZip-compatible interface
 * so ZipFSElement in sync.ts can consume it without changes.
 */
class TarAsZip {
  files: Record<string, { dir: boolean; name: string; async(type: "text"): Promise<string> }> = {};

  constructor(entries: Map<string, { content: string; isDir: boolean }>) {
    for (const [name, entry] of entries) {
      const content = entry.content;
      this.files[name] = {
        dir: entry.isDir,
        name,
        async(_type: "text") {
          return content;
        },
      };
    }
  }

  /** Return a filtered view containing only entries under the given prefix, with relative paths. */
  folder(prefix: string): TarAsZip | null {
    const normalized = prefix.endsWith("/") ? prefix : prefix + "/";
    const sub = new TarAsZip(new Map());
    for (const [name, file] of Object.entries(this.files)) {
      if (name.startsWith(normalized)) {
        const relative = name.slice(normalized.length);
        if (relative) {
          sub.files[relative] = { ...file, name: relative };
        }
      }
    }
    return Object.keys(sub.files).length > 0 ? sub : null;
  }
}

async function parseTarResponse(response: Response): Promise<TarAsZip> {
  const buffer = Buffer.from(await response.arrayBuffer());
  const entries = new Map<string, { content: string; isDir: boolean }>();
  const ex = extract();

  return new Promise((resolve, reject) => {
    ex.on("entry", (header, stream, next) => {
      const chunks: Buffer[] = [];
      stream.on("data", (chunk: Buffer) => chunks.push(chunk));
      stream.on("end", () => {
        entries.set(header.name, {
          content: Buffer.concat(chunks).toString("utf-8"),
          isDir: header.type === "directory",
        });
        next();
      });
      stream.on("error", reject);
      stream.resume();
    });
    ex.on("finish", () => resolve(new TarAsZip(entries)));
    ex.on("error", reject);
    Readable.from(buffer).pipe(ex);
  });
}

export async function downloadZip(
  workspace: Workspace,
  plainSecrets: boolean | undefined,
  skipVariables?: boolean,
  skipResources?: boolean,
  skipResourceTypes?: boolean,
  skipSecrets?: boolean,
  includeSchedules?: boolean,
  includeTriggers?: boolean,
  includeUsers?: boolean,
  includeGroups?: boolean,
  includeSettings?: boolean,
  includeKey?: boolean,
  skipWorkspaceDependencies?: boolean,
  defaultTs?: "bun" | "deno"
): Promise<JSZip | TarAsZip | undefined> {
  const requestHeaders = new Headers();
  requestHeaders.set("Authorization", "Bearer " + workspace.token);
  requestHeaders.set("Content-Type", "application/octet-stream");

  const extraHeaders = getHeaders();
  if (extraHeaders) {
    for (const [key, value] of Object.entries(extraHeaders)) {
      requestHeaders.set(key, value);
    }
  }

  const includeWorkspaceDependenciesValue = !(skipWorkspaceDependencies ?? false);
  const baseParams = `&plain_secret=${plainSecrets ?? false
    }&skip_variables=${skipVariables ?? false}&skip_resources=${skipResources ?? false
    }&skip_secrets=${skipSecrets ?? false}&include_schedules=${includeSchedules ?? false
    }&include_triggers=${includeTriggers ?? false}&include_users=${includeUsers ?? false
    }&include_groups=${includeGroups ?? false}&include_settings=${includeSettings ?? false
    }&include_key=${includeKey ?? false}&include_workspace_dependencies=${includeWorkspaceDependenciesValue}&default_ts=${defaultTs ?? "bun"}&skip_resource_types=${skipResourceTypes ?? false}&settings_version=v2`;

  const baseUrl = workspace.remote + "api/w/" + workspace.workspaceId + "/workspaces/tarball?";

  // Try zip first (standard format), fall back to tar if zip is not supported
  const zipUrl = baseUrl + "archive_type=zip" + baseParams;
  const zipResponse = await fetch(zipUrl, { headers: requestHeaders, method: "GET" });

  if (zipResponse.ok) {
    log.debug("Downloaded zip archive successfully");
    const blob = await zipResponse.blob();
    return await JSZip.loadAsync((await blob.arrayBuffer()) as any);
  }

  const body = await zipResponse.text();

  // If zip format is not supported (backend compiled without zip feature), try tar
  if (zipResponse.status === 400 && body.includes("Invalid Archive Type")) {
    log.debug("Zip archive not supported by backend, falling back to tar");
    const tarUrl = baseUrl + "archive_type=tar" + baseParams;
    const tarResponse = await fetch(tarUrl, { headers: requestHeaders, method: "GET" });

    if (tarResponse.ok) {
      log.debug("Downloaded tar archive successfully");
      return await parseTarResponse(tarResponse);
    }

    const tarBody = await tarResponse.text();
    log.info(colors.red(`Failed to request tarball from API: ${tarResponse.status} ${tarResponse.statusText}`));
    if (tarBody) log.info(colors.red(tarBody));
    return process.exit(1);
  }

  if (zipResponse.status === 404 || body.includes("no rows returned")) {
    log.info(colors.red(`Workspace '${workspace.workspaceId}' not found on ${workspace.remote}. Please check your --workspace and try again.`));
  } else {
    log.info(colors.red(`Failed to request tarball from API: ${zipResponse.status} ${zipResponse.statusText}`));
    if (body) log.info(colors.red(body));
  }
  return process.exit(1);
}

function stub(_opts: GlobalOptions & { override: boolean }, _dir: string) {
  console.log(
    colors.red.underline(
      'Pull is deprecated. Use "sync pull --raw" instead. See <TODO_LINK_HERE> for more information.'
    )
  );
}

const command = new Command()
  .description(
    "Pull all definitions in the current workspace from the API and write them to disk."
  )
  .arguments("<dir:string>")
  .action(stub as any);

export default command;
