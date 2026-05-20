import { colors } from "@cliffy/ansi/colors";
import { createServer, type Socket } from "node:net";
import { randomBytes } from "node:crypto";
import * as getPort from "get-port";
import { BackendError, createPreHashedPassword } from "pg-gateway";
import { fromNodeSocket } from "pg-gateway/node";

import * as wmill from "../../../gen/services.gen.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { pollJobWithQueueLogging } from "../../utils/job_polling.ts";
import {
  buildEmptyQueryResponse,
  buildQueryResponse,
  buildReadyForQuery,
  concat,
  type RawOutputEnvelope,
} from "./pg_wire.ts";

const DEFAULT_USER = "wmill";
const DEFAULT_PORT_RANGE_START = 5433;
const DEFAULT_PORT_RANGE_END = 5500;

export interface ServeOpts extends GlobalOptions {
  port?: number;
  host?: string;
}

export interface ServeHandle {
  host: string;
  port: number;
  user: string;
  password: string;
  /** Build a `postgresql://…/<dbName>` URL for a given datatable name. */
  connectionString: (datatableName: string) => string;
  close: () => Promise<void>;
}

/**
 * Bind the Postgres-wire proxy and return once it's accepting connections.
 * Each client connection picks its target datatable via the standard
 * Postgres `database` startup parameter (the `…/dbname` segment of the URL),
 * so the same listener can multiplex any number of datatables — and clients
 * like pgAdmin list them as separate databases instead of one shared "main".
 *
 * Caller owns `handle.close()`. Used by both `serve` (block on signal) and
 * `psql` (block on child process exit).
 */
export async function startServe(opts: ServeOpts): Promise<ServeHandle> {
  const host = opts.host ?? "127.0.0.1";

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const port =
    opts.port ??
    (await (getPort as any).default({
      port: (getPort as any).portNumbers(
        DEFAULT_PORT_RANGE_START,
        DEFAULT_PORT_RANGE_END,
      ),
    }));

  const password = randomBytes(12).toString("hex");
  const preHashedPassword = await createPreHashedPassword(
    DEFAULT_USER,
    password,
  );

  const server = createServer((socket) => {
    handleConnection(socket, workspace.workspaceId, preHashedPassword)
      .catch((err) => {
        log.warn(
          colors.yellow(
            `connection error: ${err instanceof Error ? err.message : err}`,
          ),
        );
        try {
          socket.destroy();
        } catch {
          // ignore
        }
      });
  });

  server.on("error", (err) => {
    log.error(`server error: ${err.message}`);
    process.exitCode = 1;
  });

  await new Promise<void>((resolve) => server.listen(port, host, () => resolve()));

  return {
    host,
    port,
    user: DEFAULT_USER,
    password,
    connectionString: (datatableName: string) =>
      `postgresql://${DEFAULT_USER}:${password}@${host}:${port}/${datatableName}`,
    close: () =>
      new Promise<void>((resolve) => {
        server.close(() => resolve());
        // Force-resolve if open client connections keep the server from closing.
        setTimeout(resolve, 1000).unref();
      }),
  };
}

export async function serve(opts: ServeOpts): Promise<void> {
  const handle = await startServe(opts);

  // List available datatables so the user sees what they can `\c` into.
  const workspace = await resolveWorkspace(opts);
  let datatableNames: string[] = [];
  try {
    const items = await wmill.listDataTables({
      workspace: workspace.workspaceId,
    });
    datatableNames = items.map((x) => x.name);
  } catch {
    // Listing is best-effort — the proxy still serves whatever the client asks for.
  }

  log.info(colors.gray(`Serving datatables on ${handle.host}:${handle.port} via Postgres wire protocol`));
  log.info("");
  if (datatableNames.length > 0) {
    log.info(colors.gray("Available datatables:"));
    for (const name of datatableNames) {
      log.info(`  ${colors.bold("psql")} '${handle.connectionString(name)}'`);
    }
  } else {
    log.info(
      `  ${colors.bold("psql")} '${handle.connectionString("<datatable_name>")}'`,
    );
  }
  log.info("");
  log.info(colors.gray("Press Ctrl+C to stop."));

  const shutdown = () => {
    log.info(colors.gray("\nShutting down..."));
    handle.close().finally(() => process.exit(0));
    // Force-exit if connections linger.
    setTimeout(() => process.exit(0), 1000).unref();
  };
  process.once("SIGINT", shutdown);
  process.once("SIGTERM", shutdown);

  await new Promise<void>(() => {});
}

async function handleConnection(
  socket: Socket,
  workspaceId: string,
  preHashedPassword: string,
): Promise<void> {
  await fromNodeSocket(socket, {
    auth: {
      method: "md5",
      // Same pre-hashed value regardless of the username the client sends.
      // Postgres's md5 auth derives preHashedPassword from `password + username`,
      // so a client connecting as anything other than DEFAULT_USER will produce
      // a different client-side hash and fail validation — effectively locking
      // the listener to DEFAULT_USER.
      getPreHashedPassword: async () => preHashedPassword,
    },
    // pg-gateway 0.3.0-beta.4 declares `onQuery` in its TypeScript types but
    // never actually invokes it — every frontend message goes through
    // `onMessage`, and unhandled codes fall through to a default handler that
    // returns "Message code not yet implemented". We handle the Query ('Q')
    // message ourselves and only intercept post-auth messages so the library
    // still drives the startup/auth handshake.
    onMessage: async (data, state) => {
      if (state.step !== "ReadyForQuery") return undefined;
      const code = data[0];
      if (code === FE_TERMINATE) {
        return undefined; // let pg-gateway close the connection
      }
      // Standard Postgres startup includes a `database` parameter — that's the
      // path segment in the URL (`postgresql://…/<dbname>`). pg-gateway exposes
      // it under clientParams; we use it to route the query to the matching
      // datatable. Falling back to the user name matches libpq's own default.
      const datatableName =
        state.clientParams?.database ?? state.clientParams?.user ?? "main";
      if (code === FE_QUERY) {
        const query = readSimpleQuery(data);
        try {
          return await runQuery(workspaceId, datatableName, query);
        } catch (err) {
          return concat([
            BackendError.create({
              severity: "ERROR",
              code: "XX000",
              message: err instanceof Error ? err.message : String(err ?? "unknown error"),
            }).flush(),
            buildReadyForQuery("I"),
          ]);
        }
      }
      // Extended query protocol (Parse/Bind/Describe/Execute/Sync) and other
      // codes aren't implemented yet — return a clean error so the client
      // doesn't see "Message code not yet implemented" and can keep going.
      return concat([
        BackendError.create({
          severity: "ERROR",
          code: "0A000", // feature_not_supported
          message: `Postgres message code 0x${code.toString(16).padStart(2, "0")} is not supported by 'wmill datatable serve' (simple Query only)`,
        }).flush(),
        buildReadyForQuery("I"),
      ]);
    },
  });
}

const FE_QUERY = 0x51; // 'Q'
const FE_TERMINATE = 0x58; // 'X'

/**
 * Decode a simple-query message: 'Q', INT32 length (incl. itself), then a
 * null-terminated UTF-8 string.
 */
function readSimpleQuery(data: Uint8Array): string {
  const length =
    (data[1] << 24) | (data[2] << 16) | (data[3] << 8) | data[4];
  // Strip the trailing null byte.
  const queryBytes = data.subarray(5, 1 + length - 1);
  return new TextDecoder().decode(queryBytes);
}

async function runQuery(
  workspaceId: string,
  datatableName: string,
  query: string,
): Promise<Uint8Array> {
  const trimmed = query.trim();
  if (trimmed.length === 0) {
    return concat([buildEmptyQueryResponse(), buildReadyForQuery("I")]);
  }

  // The annotation parser stops at the first non-blank line that isn't a
  // `--` comment, so prepending guarantees it's honored even if the client
  // sent leading comments of its own.
  const content = `-- raw_output\n${query}`;

  const jobId = await wmill.runScriptPreview({
    workspace: workspaceId,
    requestBody: {
      content,
      language: "postgresql",
      args: { database: `datatable://${datatableName}` },
    },
  });

  const { result, success } = await pollJobWithQueueLogging(workspaceId, jobId);

  if (!success) {
    return concat([
      BackendError.create({
        severity: "ERROR",
        code: "XX000",
        message: extractErrorMessage(result),
      }).flush(),
      buildReadyForQuery("I"),
    ]);
  }

  const envelope = coerceEnvelope(result);
  return buildQueryResponse(envelope);
}

function extractErrorMessage(result: unknown): string {
  if (typeof result === "string") return result;
  if (result && typeof result === "object") {
    const obj = result as Record<string, unknown>;
    const err = obj.error;
    if (err && typeof err === "object") {
      const errObj = err as Record<string, unknown>;
      if (typeof errObj.message === "string") return errObj.message;
      if (typeof errObj.name === "string") return errObj.name;
    }
    if (typeof obj.message === "string") return obj.message;
  }
  return JSON.stringify(result);
}

/**
 * Validate that the worker returned the {columns, rows} envelope we asked for.
 * If the shape doesn't match (older backend, unexpected error shape), fall back
 * to an empty result — better than throwing inside the wire-protocol layer.
 */
function coerceEnvelope(result: unknown): RawOutputEnvelope {
  if (
    result &&
    typeof result === "object" &&
    "columns" in result &&
    "rows" in result &&
    Array.isArray((result as any).columns) &&
    Array.isArray((result as any).rows)
  ) {
    return result as RawOutputEnvelope;
  }
  return { columns: [], rows: [] };
}
