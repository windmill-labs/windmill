import { Buffer } from "node:buffer";
import { readFile, writeFile } from "node:fs/promises";
import { basename } from "node:path";

import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { formatTimestamp } from "../../utils/utils.ts";

function formatBytes(n: number | undefined): string {
  if (n == null) return "-";
  if (n < 1024) return `${n}B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)}K`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)}M`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)}G`;
}

async function listStorages(
  opts: GlobalOptions & { json?: boolean }
) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const names = await wmill.getSecondaryStorageNames({
    workspace: workspace.workspaceId,
    includeDefault: true,
  });

  if (opts.json) {
    console.log(JSON.stringify(names));
    return;
  }
  if (names.length === 0) {
    log.info("No object storage configured for this workspace.");
    return;
  }
  for (const name of names) {
    console.log(name === "_default_" ? `${name}  ${colors.dim("(default)")}` : name);
  }
}

async function listFiles(
  opts: GlobalOptions & {
    json?: boolean;
    maxKeys?: number;
    marker?: string;
    storage?: string;
  },
  prefix?: string
) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const result = await wmill.listStoredFiles({
    workspace: workspace.workspaceId,
    maxKeys: opts.maxKeys ?? 100,
    marker: opts.marker,
    prefix,
    storage: opts.storage,
  });

  if (opts.json) {
    console.log(JSON.stringify(result));
    return;
  }
  const files = result.windmill_large_files ?? [];
  if (files.length === 0) {
    log.info("No files found.");
    return;
  }
  new Table()
    .header(["Key"])
    .padding(2)
    .border(true)
    .body(files.map((f) => [f.s3]))
    .render();
  if (result.next_marker) {
    log.info(`\nMore results available. Use --marker '${result.next_marker}' to paginate.`);
  }
}

async function upload(
  opts: GlobalOptions & {
    storage?: string;
    contentType?: string;
    contentDisposition?: string;
  },
  localPath: string,
  fileKey: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const buf = await readFile(localPath);
  // Wrap Node Buffer in a Blob for the SDK request body.
  const blob = new Blob([buf], { type: opts.contentType ?? "application/octet-stream" });

  await wmill.fileUpload({
    workspace: workspace.workspaceId,
    fileKey,
    storage: opts.storage,
    contentType: opts.contentType,
    contentDisposition: opts.contentDisposition,
    requestBody: blob,
  });
  log.info(colors.green(`Uploaded ${localPath} -> ${fileKey}`));
}

async function download(
  opts: GlobalOptions & { storage?: string; stdout?: boolean },
  fileKey: string,
  outputPath?: string
) {
  if (opts.stdout) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // The generated request layer (cli/gen/core/request.ts:getResponseBody)
  // routes by Content-Type: binary types → Blob, text/* → string, JSON → object.
  // The generated return type is `Blob | File`, which is wrong for non-binary
  // responses, so widen to unknown before normalizing.
  const body: unknown = await wmill.fileDownload({
    workspace: workspace.workspaceId,
    fileKey,
    storage: opts.storage,
  });
  let buf: Buffer;
  if (typeof body === "string") {
    buf = Buffer.from(body, "utf-8");
  } else if (body instanceof Blob) {
    buf = Buffer.from(await body.arrayBuffer());
  } else if (body instanceof ArrayBuffer) {
    buf = Buffer.from(body);
  } else if (body == null) {
    buf = Buffer.alloc(0);
  } else {
    buf = Buffer.from(JSON.stringify(body), "utf-8");
  }

  if (opts.stdout) {
    process.stdout.write(buf);
    return;
  }
  const dest = outputPath ?? basename(fileKey);
  await writeFile(dest, buf);
  log.info(colors.green(`Downloaded ${fileKey} -> ${dest}`));
}

async function del(
  opts: GlobalOptions & { storage?: string; yes?: boolean },
  fileKey: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!opts.yes) {
    const confirmed = await Confirm.prompt({
      message: `Delete '${fileKey}' from object storage${opts.storage ? ` (storage: ${opts.storage})` : ""}?`,
      default: false,
    });
    if (!confirmed) {
      log.info("Aborted.");
      return;
    }
  }

  await wmill.deleteS3File({
    workspace: workspace.workspaceId,
    fileKey,
    storage: opts.storage,
  });
  log.info(colors.green(`Deleted ${fileKey}`));
}

async function move(
  opts: GlobalOptions & { storage?: string },
  srcFileKey: string,
  destFileKey: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await wmill.moveS3File({
    workspace: workspace.workspaceId,
    srcFileKey,
    destFileKey,
    storage: opts.storage,
  });
  log.info(colors.green(`Moved ${srcFileKey} -> ${destFileKey}`));
}

async function info(
  opts: GlobalOptions & { json?: boolean; storage?: string },
  fileKey: string
) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const metadata = await wmill.loadFileMetadata({
    workspace: workspace.workspaceId,
    fileKey,
    storage: opts.storage,
  });

  if (opts.json) {
    console.log(JSON.stringify(metadata));
    return;
  }
  console.log(colors.bold("Key:") + " " + fileKey);
  console.log(colors.bold("Size:") + " " + formatBytes(metadata.size_in_bytes));
  console.log(colors.bold("Mime:") + " " + (metadata.mime_type ?? "-"));
  console.log(
    colors.bold("Last Modified:") + " " +
    (metadata.last_modified ? formatTimestamp(metadata.last_modified) : "-")
  );
  if (metadata.expires) {
    console.log(colors.bold("Expires:") + " " + formatTimestamp(metadata.expires));
  }
  if (metadata.version_id) {
    console.log(colors.bold("Version Id:") + " " + metadata.version_id);
  }
}

async function preview(
  opts: GlobalOptions & {
    storage?: string;
    bytesFrom?: number;
    bytesLength?: number;
    csvSeparator?: string;
    csvHeader?: boolean;
    mime?: string;
  },
  fileKey: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // Backend requires both byte fields; mirror the frontend's defaults
  // (frontend/src/lib/components/S3FilePickerInner.svelte) for an interactive
  // peek so the user gets useful output without passing flags.
  const result = await wmill.loadFilePreview({
    workspace: workspace.workspaceId,
    fileKey,
    storage: opts.storage,
    fileMimeType: opts.mime,
    readBytesFrom: opts.bytesFrom ?? 0,
    readBytesLength: opts.bytesLength ?? 128 * 1024,
    csvSeparator: opts.csvSeparator,
    csvHasHeader: opts.csvHeader,
  });

  if (result.msg) {
    log.info(colors.yellow(result.msg));
  }
  if (result.content != null) {
    process.stdout.write(result.content);
    if (!result.content.endsWith("\n")) process.stdout.write("\n");
  }
}

const command = new Command()
  .alias("s3")
  .description("Object storage (S3) related commands. Operates on the workspace's default object storage; use --storage to target a configured secondary storage.")
  .action(listStorages as any)
  .command(
    "list",
    "List configured object storages for the workspace (default + secondary)."
  )
  .option("--json", "Output as JSON (for piping to jq)")
  .action(listStorages as any)
  .command(
    "files",
    "List files in an object storage. Optionally filter by prefix."
  )
  .alias("ls")
  .arguments("[prefix:string]")
  .option("--json", "Output as JSON (for piping to jq)")
  .option("--max-keys <maxKeys:number>", "Page size (default 100)")
  .option("--marker <marker:string>", "Pagination marker from a previous response")
  .option("--storage <storage:string>", "Secondary storage name (omit for the workspace default)")
  .action(listFiles as any)
  .command(
    "upload",
    "Upload a local file to object storage at the given file key."
  )
  .arguments("<local_path:string> <file_key:string>")
  .option("--storage <storage:string>", "Secondary storage name")
  .option("--content-type <contentType:string>", "Content-Type header to set on the object")
  .option("--content-disposition <contentDisposition:string>", "Content-Disposition header to set on the object")
  .action(upload as any)
  .command(
    "download",
    "Download an object to a local file (or stdout). Default output path is the basename of the file key in the current directory."
  )
  .arguments("<file_key:string> [output_path:string]")
  .option("--storage <storage:string>", "Secondary storage name")
  .option("--stdout", "Write file contents to stdout instead of a file")
  .action(download as any)
  .command(
    "delete",
    "Delete an object from object storage. Prompts for confirmation unless --yes is set."
  )
  .arguments("<file_key:string>")
  .option("--storage <storage:string>", "Secondary storage name")
  .option("--yes", "Skip the confirmation prompt")
  .action(del as any)
  .command(
    "move",
    "Move an object within the same storage (rename or relocate by key)."
  )
  .arguments("<src_file_key:string> <dest_file_key:string>")
  .option("--storage <storage:string>", "Secondary storage name")
  .action(move as any)
  .command(
    "info",
    "Show metadata (size, mime, last-modified) for an object."
  )
  .arguments("<file_key:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .option("--storage <storage:string>", "Secondary storage name")
  .action(info as any)
  .command(
    "preview",
    "Preview the contents of an object (text/CSV). Use --bytes-from / --bytes-length to peek at a slice of binary files."
  )
  .arguments("<file_key:string>")
  .option("--storage <storage:string>", "Secondary storage name")
  .option("--mime <mime:string>", "Override the detected mime type (e.g. text/csv)")
  .option("--bytes-from <bytesFrom:number>", "Start offset in bytes")
  .option("--bytes-length <bytesLength:number>", "Number of bytes to read")
  .option("--csv-separator <csvSeparator:string>", "CSV column separator (default ,)")
  .option("--csv-header", "Treat the first CSV row as a header")
  .action(preview as any);

export default command;
