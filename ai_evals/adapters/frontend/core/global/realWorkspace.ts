// A real Windmill workspace per global-mode attempt. The session's tools run on the backend
// (`/api/w/<ws>/ai_session_tools/<name>`), so a case's fixture has to exist there: this creates
// the workspace, seeds it through the public API, and deletes it afterwards.
import { randomUUID } from "node:crypto";
import type { WindmillBackendSettings } from "../../../../core/windmillBackendSettings";
import type { EvalCaseProfile } from "../../../../core/types";
import type {
  BenchmarkDatatableSeed,
  BenchmarkWorkspaceRunnables,
} from "../../mockBackend";
import { WindmillBackendClient } from "../../windmillBackend";
import { buildWorkspaceId } from "../../workspaceId";
import type { GlobalUserFixture } from "./globalEvalRunner";

/** Workspace id -> the token its case runs as. Read by the adapter's fetch stub. */
const activeWorkspaces = new Map<string, string>();
let adminToken: string | undefined;
let baseUrl: string | undefined;

export function realWorkspaceRoute(
  url: string,
): { url: string; token: string } | undefined {
  if (!baseUrl || !adminToken || !url.startsWith("/api/")) return undefined;
  const workspace = /^\/api\/w\/([^/?]+)/.exec(url)?.[1];
  const token = workspace
    ? activeWorkspaces.get(decodeURIComponent(workspace))
    : activeWorkspaces.size > 0
      ? adminToken
      : undefined;
  return token ? { url: `${baseUrl}${url}`, token } : undefined;
}

// Without the `postgresql` tag (a `native` worker group) datatable fixtures and SQL tools hang.
const DATATABLE_PG_URL =
  process.env.WMILL_AI_EVAL_DATATABLE_PG_URL ??
  "postgres://postgres:changeme@localhost:5432/postgres";
const ROOT_PG_RESOURCE = "$res:u/admin/ai_evals_pg_root";
const WORKSPACE_SLOT_WAIT_MS = 30 * 60_000;
const EVAL_USER_PASSWORD = "ai-evals-user";

export interface RealWorkspace {
  id: string;
  /** Settings that make the model proxy reuse this workspace instead of creating another. */
  backend: WindmillBackendSettings;
  dispose(): Promise<void>;
}

export async function createRealWorkspace(input: {
  settings: WindmillBackendSettings;
  caseId: string;
  attempt: number;
  fixtures: BenchmarkWorkspaceRunnables;
  user?: GlobalUserFixture;
  profile?: EvalCaseProfile;
}): Promise<RealWorkspace> {
  // Job ids are unique across the instance, and a case prompt can name a fixture job by its
  // id: attempts that seed the same ids take turns, each holding them until it is deleted.
  const releaseJobIds = await holdJobIds(
    (input.fixtures.jobs ?? []).flatMap((job) => (job.id ? [job.id] : [])),
  );
  const client = new WindmillBackendClient(input.settings);
  baseUrl = input.settings.baseUrl;
  const id = buildWorkspaceId(input.caseId, input.attempt);
  const api = apiFor(client, id);
  try {
    adminToken = await client.getToken();
    await createWorkspace(api, id);
  } catch (error) {
    releaseJobIds();
    throw error;
  }
  let datatableDb: string | undefined;
  const dispose = async () => {
    activeWorkspaces.delete(id);
    if (datatableDb) {
      await sql(
        api,
        ROOT_PG_RESOURCE,
        `DROP DATABASE IF EXISTS "${datatableDb}" WITH (FORCE)`,
      ).catch(() => undefined);
    }
    await api("DELETE", `/workspaces/delete/${id}`).catch(() => undefined);
    releaseJobIds();
  };
  try {
    activeWorkspaces.set(id, adminToken);
    const token = await seedUser(api, id, input.user, input.profile);
    datatableDb = await seed(api, id, input.fixtures);
    activeWorkspaces.set(id, token ?? adminToken);
    return {
      id,
      backend: { ...input.settings, workspaceOverride: id },
      dispose,
    };
  } catch (error) {
    await dispose();
    throw error;
  }
}

/** Job id -> the turn of the last attempt that asked for it. */
const jobIdTurns = new Map<string, Promise<void>>();

async function holdJobIds(ids: string[]): Promise<() => void> {
  let release!: () => void;
  const turn = new Promise<void>((resolve) => (release = resolve));
  const previous = ids.map((id) => jobIdTurns.get(id) ?? Promise.resolve());
  for (const id of ids) jobIdTurns.set(id, turn);
  await Promise.all(previous);
  return release;
}

type Api = (method: string, path: string, body?: unknown) => Promise<any>;

function apiFor(client: WindmillBackendClient, workspace: string): Api {
  return async (method, path, body) => {
    const scoped = path.startsWith("/") ? path : `/w/${workspace}/${path}`;
    const response = await client.request(scoped, {
      method,
      ...(body instanceof FormData
        ? { body }
        : body === undefined
          ? {}
          : {
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify(body),
            }),
    });
    const text = await response.text();
    if (!response.ok) {
      throw new Error(`${method} ${scoped}: ${response.status} ${text}`);
    }
    try {
      return JSON.parse(text);
    } catch {
      return text;
    }
  };
}

// CE caps the number of workspaces, so a run with more concurrent attempts than slots waits.
async function createWorkspace(api: Api, id: string): Promise<void> {
  const deadline = Date.now() + WORKSPACE_SLOT_WAIT_MS;
  for (;;) {
    try {
      await api("POST", "/workspaces/create", { id, name: id });
      return;
    } catch (error) {
      if (
        !String(error).includes("maximum number of workspaces") ||
        Date.now() > deadline
      ) {
        throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, 5_000));
    }
  }
}

// The admin running the eval is `admin` in every workspace it creates. Any other identity is a
// real member, so the backend enforces its rights as it would for that user; CE cannot create
// users, so there the case runs as the admin, with the prompt alone describing the fixture user.
async function seedUser(
  api: Api,
  workspace: string,
  user: GlobalUserFixture | undefined,
  profile: EvalCaseProfile | undefined,
): Promise<string | undefined> {
  const username = user?.username ?? "admin";
  const writable = new Set(user?.folders ?? []);
  const readable = new Set([...(user?.folders_read ?? []), ...writable]);
  const asAdmin = username === "admin" && (user?.is_admin ?? true);
  let token: string | undefined;
  if (!asAdmin) {
    const email = `${username}@ai-evals.windmill.dev`;
    try {
      await api("POST", "/users/create", {
        email,
        password: EVAL_USER_PASSWORD,
        super_admin: false,
        name: username,
      }).catch((error) => {
        if (!String(error).includes("already exists")) throw error;
      });
      await api("POST", "workspaces/add_user", {
        email,
        is_admin: user?.is_admin ?? false,
        operator: profile?.operator ?? false,
      });
      token = String(
        await api("POST", "/auth/login", { email, password: EVAL_USER_PASSWORD }),
      ).trim();
    } catch (error) {
      if (!String(error).includes("not implemented in the open-source")) throw error;
      console.warn(
        `[ai-evals] ${workspace}: CE cannot create the fixture user "${username}"; the backend runs this case as the admin.`,
      );
    }
  }
  if (profile?.operator && profile.operatorSettings) {
    await api("POST", "workspaces/operator_settings", profile.operatorSettings);
  }
  for (const folder of readable) {
    await api("POST", "folders/create", {
      name: folder,
      ...(asAdmin ? {} : { extra_perms: { [`u/${username}`]: writable.has(folder) } }),
    });
  }
  return token;
}

/** Seeds every fixture kind; returns the scratch database a datatable fixture created. */
async function seed(
  api: Api,
  workspace: string,
  fixtures: BenchmarkWorkspaceRunnables,
): Promise<string | undefined> {
  for (const rt of fixtures.resourceTypes ?? []) {
    await api("POST", "resources/type/create", {
      name: rt.name,
      schema: rt.schema ?? {},
      description: rt.description ?? "",
    });
  }
  for (const script of fixtures.scripts ?? []) {
    await api("POST", "scripts/create", {
      path: script.path,
      summary: script.summary,
      description: script.description ?? "",
      content: script.content,
      language: script.language,
      schema: script.schema ?? {},
    });
  }
  for (const flow of fixtures.flows ?? []) {
    await api("POST", "flows/create", {
      path: flow.path,
      summary: flow.summary,
      description: flow.description ?? "",
      value: flow.value,
      schema: flow.schema ?? {},
    });
  }
  for (const app of fixtures.apps ?? []) {
    await seedApp(api, app);
  }
  for (const variable of fixtures.variables ?? []) {
    await api("POST", "variables/create", {
      path: variable.path,
      value: variable.value,
      is_secret: variable.is_secret,
      description: variable.description ?? "",
      ...(variable.labels ? { labels: variable.labels } : {}),
    });
  }
  for (const resource of fixtures.resources ?? []) {
    await api("POST", "resources/create", {
      path: resource.path,
      resource_type: resource.resource_type,
      value: resource.value ?? {},
      description: resource.description ?? "",
    });
  }
  await seedAiProviders(api, fixtures.aiProviders ?? []);
  for (const job of fixtures.jobs ?? []) {
    await seedJob(api, job);
  }
  if (fixtures.ducklakes?.length) {
    await api("POST", "workspaces/edit_ducklake_config", {
      settings: {
        ducklakes: Object.fromEntries(
          fixtures.ducklakes.map((name) => [
            name,
            {
              catalog: { resource_type: "instance", resource_path: `${workspace}_${name}` },
              storage: { storage: null, path: name },
            },
          ]),
        ),
      },
    });
  }
  await seedDataMetrics(api, fixtures.dataMetrics ?? []);
  const datatables = fixtures.datatables ?? [];
  if (datatables.length > 1) {
    throw new Error("A global fixture seeds at most one datatable.");
  }
  return datatables[0] ? await seedDatatable(api, workspace, datatables[0]) : undefined;
}

async function seedApp(
  api: Api,
  app: NonNullable<BenchmarkWorkspaceRunnables["apps"]>[number],
): Promise<void> {
  const policy = { execution_mode: "publisher", triggerables_v2: {} };
  if (app.rawApp === false) {
    await api("POST", "apps/create", {
      path: app.path,
      summary: app.summary,
      value: app.value,
      policy,
    });
    return;
  }
  const form = new FormData();
  form.append(
    "app",
    JSON.stringify({
      path: app.path,
      summary: app.summary,
      value: { ...app.value, data: app.value.data ?? { tables: [] } },
      policy,
    }),
  );
  form.append("js", "");
  form.append("css", "");
  await api("POST", "apps/create_raw", form);
}

async function seedAiProviders(
  api: Api,
  providers: NonNullable<BenchmarkWorkspaceRunnables["aiProviders"]>,
): Promise<void> {
  if (!providers.length) return;
  const config: Record<string, unknown> = {};
  let defaultModel: { model: string; provider: string } | undefined;
  for (const provider of providers) {
    await api("POST", "resources/create", {
      path: provider.path,
      resource_type: provider.kind,
      value: {
        api_key: "ai-evals-placeholder",
        ...(provider.base_url ? { base_url: provider.base_url } : {}),
      },
    });
    const models = provider.configuredModels ?? provider.models ?? [];
    config[provider.kind] = { resource_path: provider.path, models };
    if (provider.isDefault && models[0]) {
      defaultModel = { model: models[0], provider: provider.kind };
    }
  }
  await api("POST", "workspaces/edit_copilot_config", {
    providers: config,
    ...(defaultModel ? { default_model: defaultModel } : {}),
  });
}

// A completed run with the fixture's id, path, args, logs and outcome. It is a preview job run
// by the admin, so `createdBy` and a `script` job kind are not reproduced.
async function seedJob(
  api: Api,
  job: NonNullable<BenchmarkWorkspaceRunnables["jobs"]>[number],
): Promise<void> {
  const args = job.args ?? {};
  const content = [
    `export async function main(${Object.keys(args).join(", ")}) {`,
    `  console.log(${JSON.stringify(job.logs ?? "")});`,
    job.success === false
      ? `  throw new Error(${JSON.stringify(lastLine(job.logs) ?? "Job failed")});`
      : `  return ${JSON.stringify(job.result ?? null)};`,
    `}`,
  ].join("\n");
  const id = job.id ?? randomUUID();
  await api("POST", `jobs/run_wait_result/preview?job_id=${id}`, {
    content,
    language: "bun",
    path: job.scriptPath,
    args,
  }).catch((error) => {
    // A failing fixture run answers with its error, which is the point.
    if (job.success !== false) throw error;
  });
}

function lastLine(text: string | undefined): string | undefined {
  return text?.trim().split("\n").pop();
}

// Metrics are catalogued from a DuckDB node's annotations when it is deployed.
async function seedDataMetrics(
  api: Api,
  metrics: NonNullable<BenchmarkWorkspaceRunnables["dataMetrics"]>,
): Promise<void> {
  const byScript = new Map<string, typeof metrics>();
  for (const metric of metrics) {
    const key = `${metric.script_path}\n${metric.table_path}`;
    byScript.set(key, [...(byScript.get(key) ?? []), metric]);
  }
  for (const [key, entries] of byScript) {
    const [scriptPath, tablePath] = key.split("\n");
    // `<lake>/<schema>.<table>`, as the catalog canonicalizes `ducklake://<lake>/<table>`.
    const [lake, qualified] = tablePath.split("/");
    const table = qualified.split(".").pop();
    const lines = [`-- materialize ducklake://${lake}/${table}`];
    for (const metric of entries) {
      lines.push(
        metric.kind === "measure"
          ? `-- measure ${metric.name} = ${metric.expr}${metric.filter ? ` where ${metric.filter}` : ""}`
          : `-- dimension ${metric.name} = ${metric.expr}`,
      );
    }
    lines.push("SELECT 1");
    await api("POST", "scripts/create", {
      path: scriptPath,
      summary: "",
      description: "",
      content: lines.join("\n"),
      language: "duckdb",
      schema: {},
    });
  }
}

async function seedDatatable(
  api: Api,
  workspace: string,
  seed: BenchmarkDatatableSeed,
): Promise<string> {
  const root = new URL(DATATABLE_PG_URL);
  const database = `${workspace.replace(/-/g, "_")}_dt`.slice(0, 63);
  const resource = (dbname: string) => ({
    host: root.hostname,
    port: Number(root.port || 5432),
    user: decodeURIComponent(root.username),
    password: decodeURIComponent(root.password),
    dbname,
    sslmode: root.searchParams.get("sslmode") ?? "disable",
  });
  await api("POST", "resources/create", {
    path: "u/admin/ai_evals_pg_root",
    resource_type: "postgresql",
    value: resource(root.pathname.slice(1) || "postgres"),
  });
  await sql(api, ROOT_PG_RESOURCE, `CREATE DATABASE "${database}"`);
  await api("POST", "resources/create", {
    path: "u/admin/ai_evals_datatable",
    resource_type: "postgresql",
    value: resource(database),
  });
  await api("POST", "workspaces/edit_datatable_config", {
    settings: {
      datatables: {
        [seed.datatable_name]: {
          database: { resource_type: "postgresql", resource_path: "u/admin/ai_evals_datatable" },
          ...(seed.migrations_enabled ? { migrations_enabled: true } : {}),
        },
      },
    },
  });
  const target = `datatable://${seed.datatable_name}`;
  for (const migration of seed.migrations ?? []) {
    await api("POST", `workspaces/upsert_datatable_migration/${seed.datatable_name}`, {
      timestamp: migration.timestamp,
      name: migration.name,
      code_up: migration.code_up,
      ...(migration.code_down ? { code_down: migration.code_down } : {}),
    });
    if (migration.applied) {
      await api(
        "POST",
        `workspaces/run_datatable_migrations/${seed.datatable_name}?only=${migration.timestamp}`,
      );
    }
  }
  const statements: string[] = [];
  for (const [schema, tables] of Object.entries(seed.schemas)) {
    statements.push(`CREATE SCHEMA IF NOT EXISTS "${schema}"`);
    for (const [table, { columns, rows }] of Object.entries(tables)) {
      const name = `"${schema}"."${table}"`;
      statements.push(
        `CREATE TABLE IF NOT EXISTS ${name} (${Object.entries(columns)
          .map(([column, type]) => `"${column}" ${type}`)
          .join(", ")})`,
      );
      for (const row of rows ?? []) {
        const keys = Object.keys(row);
        statements.push(
          `INSERT INTO ${name} (${keys.map((k) => `"${k}"`).join(", ")}) VALUES (${keys
            .map((k) => literal(row[k]))
            .join(", ")})`,
        );
      }
    }
  }
  await sql(api, target, statements.join(";\n"));
  return database;
}

function literal(value: unknown): string {
  if (value === null || value === undefined) return "NULL";
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  const text = typeof value === "string" ? value : JSON.stringify(value);
  return `'${text.replace(/'/g, "''")}'`;
}

async function sql(api: Api, database: string, content: string): Promise<void> {
  const result = await api("POST", "jobs/run_wait_result/preview", {
    content,
    language: "postgresql",
    args: { database },
  });
  if (result && typeof result === "object" && "error" in result) {
    throw new Error(`seeding SQL failed: ${JSON.stringify(result.error)}`);
  }
}
