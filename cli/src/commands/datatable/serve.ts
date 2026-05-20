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
  buildBindComplete,
  buildCloseComplete,
  buildEmptyQueryResponse,
  buildExecuteResponse,
  buildNoData,
  buildParameterDescription,
  buildParseComplete,
  buildQueryResponse,
  buildReadyForQuery,
  buildRowDescription,
  concat,
  type RawOutputEnvelope,
} from "./pg_wire.ts";
import {
  queryTouchesVirtualDatabaseCatalog,
  virtualizeDatabaseCatalogQuery,
} from "./virtual_catalog.ts";

const DEFAULT_USER = "wmill";
const DEFAULT_PORT_RANGE_START = 5433;
const DEFAULT_PORT_RANGE_END = 5500;

export interface ServeOpts extends GlobalOptions {
  port?: number;
  host?: string;
  password?: string;
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

interface PreparedStatementState {
  query: string;
  parameterTypeOids: number[];
}

interface PortalState {
  query: string;
  resultFormatCodes: number[];
  parameterTypeOids: number[];
  parameterSqlExpressions: string[];
  envelope?: RawOutputEnvelope;
  rowOffset: number;
  sentRowDescription: boolean;
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

  const password = opts.password ?? randomBytes(12).toString("hex");
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
      `postgresql://${DEFAULT_USER}:${encodeURIComponent(password)}@${host}:${port}/${encodeURIComponent(datatableName)}`,
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
  const preparedStatements = new Map<string, PreparedStatementState>();
  const portals = new Map<string, PortalState>();
  let waitingForSyncAfterError = false;

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
      if (waitingForSyncAfterError) {
        if (code === FE_SYNC) {
          waitingForSyncAfterError = false;
          return buildReadyForQuery("I");
        }
        return new Uint8Array(0);
      }

      try {
        switch (code) {
          case FE_QUERY:
            return await runQuery(workspaceId, datatableName, readSimpleQuery(data));
          case FE_PARSE:
            return handleParseMessage(data, preparedStatements, portals);
          case FE_BIND:
            return handleBindMessage(data, preparedStatements, portals);
          case FE_DESCRIBE:
            return await handleDescribeMessage(
              data,
              preparedStatements,
              portals,
              (query) => runQueryEnvelope(workspaceId, datatableName, query),
            );
          case FE_EXECUTE:
            return await handleExecuteMessage(
              data,
              portals,
              (query) => runQueryEnvelope(workspaceId, datatableName, query),
            );
          case FE_CLOSE:
            return handleCloseMessage(data, preparedStatements, portals);
          case FE_FLUSH:
            return new Uint8Array(0);
          case FE_SYNC:
            return buildReadyForQuery("I");
          default:
            throw createPgProtocolError(
              `Postgres message code 0x${code.toString(16).padStart(2, "0")} is not supported by 'wmill datatable serve'`,
              "0A000",
            );
        }
      } catch (err) {
        const includeReadyForQuery = code === FE_QUERY;
        waitingForSyncAfterError = !includeReadyForQuery;
        return buildPgErrorResponse(err, includeReadyForQuery);
      }
    },
  });
}

const FE_QUERY = 0x51; // 'Q'
const FE_PARSE = 0x50; // 'P'
const FE_BIND = 0x42; // 'B'
const FE_CLOSE = 0x43; // 'C'
const FE_DESCRIBE = 0x44; // 'D'
const FE_EXECUTE = 0x45; // 'E'
const FE_FLUSH = 0x48; // 'H'
const FE_SYNC = 0x53; // 'S'
const FE_TERMINATE = 0x58; // 'X'

type PgProtocolError = Error & { pgCode?: string };

const textDecoder = new TextDecoder();

class FrontendMessageReader {
  constructor(
    private readonly data: Uint8Array,
    private offset = 5,
  ) {}

  byte(): number {
    return this.data[this.offset++] ?? 0;
  }

  int16(): number {
    const view = new DataView(
      this.data.buffer,
      this.data.byteOffset,
      this.data.byteLength,
    );
    const value = view.getInt16(this.offset);
    this.offset += 2;
    return value;
  }

  int32(): number {
    const view = new DataView(
      this.data.buffer,
      this.data.byteOffset,
      this.data.byteLength,
    );
    const value = view.getInt32(this.offset);
    this.offset += 4;
    return value;
  }

  cstring(): string {
    const start = this.offset;
    while (this.offset < this.data.length && this.data[this.offset] !== 0) {
      this.offset += 1;
    }
    const value = textDecoder.decode(this.data.subarray(start, this.offset));
    this.offset += 1;
    return value;
  }

  bytes(length: number): Uint8Array {
    const value = this.data.subarray(this.offset, this.offset + length);
    this.offset += length;
    return value;
  }
}

/**
 * Decode a simple-query message: 'Q', INT32 length (incl. itself), then a
 * null-terminated UTF-8 string.
 */
function readSimpleQuery(data: Uint8Array): string {
  const length =
    (data[1] << 24) | (data[2] << 16) | (data[3] << 8) | data[4];
  // Strip the trailing null byte.
  const queryBytes = data.subarray(5, 1 + length - 1);
  return textDecoder.decode(queryBytes);
}

function readParseMessage(data: Uint8Array): {
  statementName: string;
  query: string;
  parameterTypeOids: number[];
} {
  const reader = new FrontendMessageReader(data);
  const statementName = reader.cstring();
  const query = reader.cstring();
  const parameterCount = reader.int16();
  const parameterTypeOids = Array.from({ length: parameterCount }, () =>
    reader.int32(),
  );
  return { statementName, query, parameterTypeOids };
}

function readBindMessage(data: Uint8Array): {
  portalName: string;
  statementName: string;
  parameterFormatCodes: number[];
  parameters: (Uint8Array | null)[];
  resultFormatCodes: number[];
} {
  const reader = new FrontendMessageReader(data);
  const portalName = reader.cstring();
  const statementName = reader.cstring();
  const parameterFormatCodeCount = reader.int16();
  const parameterFormatCodes = Array.from(
    { length: parameterFormatCodeCount },
    () => reader.int16(),
  );
  const parameterCount = reader.int16();
  const parameters = Array.from({ length: parameterCount }, () => {
    const length = reader.int32();
    return length === -1 ? null : reader.bytes(length);
  });
  const resultFormatCodeCount = reader.int16();
  const resultFormatCodes = Array.from(
    { length: resultFormatCodeCount },
    () => reader.int16(),
  );
  return {
    portalName,
    statementName,
    parameterFormatCodes,
    parameters,
    resultFormatCodes,
  };
}

function readDescribeMessage(data: Uint8Array): {
  targetType: "P" | "S";
  name: string;
} {
  const reader = new FrontendMessageReader(data);
  const targetType = String.fromCharCode(reader.byte()) as "P" | "S";
  return { targetType, name: reader.cstring() };
}

function readExecuteMessage(data: Uint8Array): {
  portalName: string;
  maxRows: number;
} {
  const reader = new FrontendMessageReader(data);
  return { portalName: reader.cstring(), maxRows: reader.int32() };
}

function readCloseMessage(data: Uint8Array): {
  targetType: "P" | "S";
  name: string;
} {
  const reader = new FrontendMessageReader(data);
  const targetType = String.fromCharCode(reader.byte()) as "P" | "S";
  return { targetType, name: reader.cstring() };
}

function handleParseMessage(
  data: Uint8Array,
  preparedStatements: Map<string, PreparedStatementState>,
  portals: Map<string, PortalState>,
): Uint8Array {
  const parse = readParseMessage(data);
  preparedStatements.set(parse.statementName, {
    query: parse.query,
    parameterTypeOids: parse.parameterTypeOids,
  });
  if (parse.statementName === "") {
    portals.delete("");
  }
  return buildParseComplete();
}

function handleBindMessage(
  data: Uint8Array,
  preparedStatements: Map<string, PreparedStatementState>,
  portals: Map<string, PortalState>,
): Uint8Array {
  const bind = readBindMessage(data);
  const statement = preparedStatements.get(bind.statementName);
  if (!statement) {
    throw createPgProtocolError(
      `prepared statement ${formatProtocolName(bind.statementName, "statement")} does not exist`,
      "26000",
    );
  }
  for (let i = 0; i < bind.parameters.length; i += 1) {
    if (bind.parameters[i] !== null && resolveParameterFormatCode(bind.parameterFormatCodes, i) !== 0) {
      throw createPgProtocolError(
        "binary parameter formats are not supported by 'wmill datatable serve'",
        "0A000",
      );
    }
  }
  if (bind.resultFormatCodes.some((code) => code !== 0)) {
    throw createPgProtocolError(
      "binary result formats are not supported by 'wmill datatable serve'",
      "0A000",
    );
  }
  portals.set(bind.portalName, {
    query: statement.query,
    resultFormatCodes: bind.resultFormatCodes,
    parameterTypeOids: statement.parameterTypeOids,
    parameterSqlExpressions: bind.parameters.map(sqlExpressionForBoundParameter),
    rowOffset: 0,
    sentRowDescription: false,
  });
  return buildBindComplete();
}

async function handleDescribeMessage(
  data: Uint8Array,
  preparedStatements: Map<string, PreparedStatementState>,
  portals: Map<string, PortalState>,
  executeQuery: (query: string) => Promise<RawOutputEnvelope>,
): Promise<Uint8Array> {
  const describe = readDescribeMessage(data);
  if (describe.targetType === "S") {
    const statement = preparedStatements.get(describe.name);
    if (!statement) {
      throw createPgProtocolError(
        `prepared statement ${formatProtocolName(describe.name, "statement")} does not exist`,
        "26000",
      );
    }
    if (
      statement.parameterTypeOids.length === 0 &&
      isQueryLikelySafeToDescribe(statement.query)
    ) {
      const envelope = await executeQuery(statement.query);
      return concat([
        buildParameterDescription(statement.parameterTypeOids),
        envelope.columns.length > 0
          ? buildRowDescription(envelope.columns)
          : buildNoData(),
      ]);
    }
    return concat([
      buildParameterDescription(statement.parameterTypeOids),
      buildNoData(),
    ]);
  }

  const portal = portals.get(describe.name);
  if (!portal) {
    throw createPgProtocolError(
      `portal ${formatProtocolName(describe.name, "portal")} does not exist`,
      "34000",
    );
  }

  if (!portal.envelope && isQueryLikelySafeToDescribe(portal.query)) {
    portal.envelope = await executeQuery(buildPortalQuery(portal));
  }
  if (portal.envelope?.columns.length) {
    portal.sentRowDescription = true;
    return buildRowDescription(portal.envelope.columns);
  }
  return buildNoData();
}

async function handleExecuteMessage(
  data: Uint8Array,
  portals: Map<string, PortalState>,
  executeQuery: (query: string) => Promise<RawOutputEnvelope>,
): Promise<Uint8Array> {
  const execute = readExecuteMessage(data);
  const portal = portals.get(execute.portalName);
  if (!portal) {
    throw createPgProtocolError(
      `portal ${formatProtocolName(execute.portalName, "portal")} does not exist`,
      "34000",
    );
  }
  if (portal.resultFormatCodes.some((code) => code !== 0)) {
    throw createPgProtocolError(
      "binary result formats are not supported by 'wmill datatable serve'",
      "0A000",
    );
  }
  if (portal.query.trim().length === 0) {
    return buildEmptyQueryResponse();
  }
  if (!portal.envelope) {
    portal.envelope = await executeQuery(buildPortalQuery(portal));
  }

  const response = buildExecuteResponse(portal.envelope, {
    includeRowDescription: !portal.sentRowDescription,
    maxRows: execute.maxRows,
    rowOffset: portal.rowOffset,
  });

  portal.sentRowDescription = true;
  if (response.suspended) {
    portal.rowOffset = response.nextRowOffset;
  } else {
    portal.envelope = undefined;
    portal.rowOffset = 0;
    portal.sentRowDescription = false;
  }

  return response.message;
}

function handleCloseMessage(
  data: Uint8Array,
  preparedStatements: Map<string, PreparedStatementState>,
  portals: Map<string, PortalState>,
): Uint8Array {
  const close = readCloseMessage(data);
  if (close.targetType === "S") {
    preparedStatements.delete(close.name);
  } else {
    portals.delete(close.name);
  }
  return buildCloseComplete();
}

function createPgProtocolError(message: string, pgCode = "XX000"): PgProtocolError {
  const err = new Error(message) as PgProtocolError;
  err.pgCode = pgCode;
  return err;
}

function resolveParameterFormatCode(formatCodes: number[], index: number): number {
  if (formatCodes.length === 0) return 0;
  if (formatCodes.length === 1) return formatCodes[0] ?? 0;
  return formatCodes[index] ?? 0;
}

function sqlExpressionForBoundParameter(value: Uint8Array | null): string {
  if (value === null) {
    return "NULL";
  }
  return sqlStringLiteral(textDecoder.decode(value));
}

function sqlStringLiteral(value: string): string {
  return `'${value.replaceAll("'", "''")}'`;
}

function buildPortalQuery(portal: PortalState): string {
  if (portal.parameterSqlExpressions.length === 0) {
    return portal.query;
  }

  const trimmedQuery = portal.query.replace(/;+\s*$/, "");
  const typeList = buildPreparedStatementTypeList(portal.parameterTypeOids);
  return [
    `PREPARE "__wmill_proxy_stmt"${typeList} AS ${trimmedQuery};`,
    `EXECUTE "__wmill_proxy_stmt"(${portal.parameterSqlExpressions.join(", ")});`,
  ].join("\n");
}

function buildPreparedStatementTypeList(parameterTypeOids: number[]): string {
  if (parameterTypeOids.length === 0) {
    return "";
  }

  const typeNames = parameterTypeOids.map(postgresTypeNameFromOid);
  if (typeNames.some((name) => name === undefined)) {
    return "";
  }

  return `(${typeNames.join(", ")})`;
}

function postgresTypeNameFromOid(oid: number): string | undefined {
  switch (oid) {
    case 16:
      return "pg_catalog.bool";
    case 17:
      return "pg_catalog.bytea";
    case 19:
      return "pg_catalog.name";
    case 20:
      return "pg_catalog.int8";
    case 21:
      return "pg_catalog.int2";
    case 23:
      return "pg_catalog.int4";
    case 25:
      return "pg_catalog.text";
    case 26:
      return "pg_catalog.oid";
    case 700:
      return "pg_catalog.float4";
    case 701:
      return "pg_catalog.float8";
    case 1042:
      return "pg_catalog.bpchar";
    case 1043:
      return "pg_catalog.varchar";
    case 1082:
      return "pg_catalog.date";
    case 1083:
      return "pg_catalog.time";
    case 1114:
      return "pg_catalog.timestamp";
    case 1184:
      return "pg_catalog.timestamptz";
    case 1266:
      return "pg_catalog.timetz";
    case 1700:
      return "pg_catalog.numeric";
    case 2950:
      return "pg_catalog.uuid";
    case 114:
      return "pg_catalog.json";
    case 3802:
      return "pg_catalog.jsonb";
    default:
      return undefined;
  }
}

function isQueryLikelySafeToDescribe(query: string): boolean {
  let normalized = query.trimStart();
  while (true) {
    if (normalized.startsWith("--")) {
      const newlineIndex = normalized.indexOf("\n");
      normalized =
        newlineIndex === -1
          ? ""
          : normalized.slice(newlineIndex + 1).trimStart();
      continue;
    }
    if (normalized.startsWith("/*")) {
      const commentEnd = normalized.indexOf("*/");
      normalized =
        commentEnd === -1
          ? ""
          : normalized.slice(commentEnd + 2).trimStart();
      continue;
    }
    break;
  }
  normalized = normalized.toLowerCase();

  return (
    normalized.startsWith("select") ||
    normalized.startsWith("with") ||
    normalized.startsWith("show") ||
    normalized.startsWith("values") ||
    normalized.startsWith("table")
  );
}

function buildPgErrorResponse(
  err: unknown,
  includeReadyForQuery: boolean,
): Uint8Array {
  const error = err instanceof Error ? err : new Error(String(err ?? "unknown error"));
  const parts = [
    BackendError.create({
      severity: "ERROR",
      code: (error as PgProtocolError).pgCode ?? "XX000",
      message: error.message,
    }).flush(),
  ];
  if (includeReadyForQuery) {
    parts.push(buildReadyForQuery("I"));
  }
  return concat(parts);
}

function formatProtocolName(name: string, kind: "portal" | "statement"): string {
  if (name.length === 0) {
    return kind === "portal" ? "<unnamed portal>" : "<unnamed statement>";
  }
  return `"${name}"`;
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

  const envelope = await runQueryEnvelope(workspaceId, datatableName, query);
  return buildQueryResponse(envelope);
}

async function runQueryEnvelope(
  workspaceId: string,
  datatableName: string,
  query: string,
): Promise<RawOutputEnvelope> {
  const trimmed = query.trim();
  if (trimmed.length === 0) {
    return { columns: [], rows: [] };
  }

  // The annotation parser stops at the first non-blank line that isn't a
  // `--` comment, so prepending guarantees it's honored even if the client
  // sent leading comments of its own.
  const content = `-- raw_output\n${await prepareQueryForDatatableProxy(
    workspaceId,
    datatableName,
    query,
  )}`;

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
    throw createPgProtocolError(extractErrorMessage(result));
  }

  return coerceEnvelope(result);
}

async function prepareQueryForDatatableProxy(
  workspaceId: string,
  datatableName: string,
  query: string,
): Promise<string> {
  if (!queryTouchesVirtualDatabaseCatalog(query)) {
    return query;
  }

  let databaseNames = [datatableName];
  if (query.toLowerCase().includes("pg_database")) {
    try {
      const items = await wmill.listDataTables({
        workspace: workspaceId,
      });
      databaseNames = Array.from(
        new Set([datatableName, ...items.map((item) => item.name)]),
      ).sort();
    } catch {
      // Fall back to the current datatable so catalog queries still stay virtual.
    }
  }

  return virtualizeDatabaseCatalogQuery(query, datatableName, databaseNames);
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
