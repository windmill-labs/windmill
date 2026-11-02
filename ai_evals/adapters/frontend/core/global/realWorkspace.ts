// A real Windmill workspace per global-mode attempt. The session's tools run on the backend
// (`/api/w/<ws>/ai_session_tools/<name>`), so a case's fixture has to exist there: this creates
// the workspace, seeds it through the public API, and deletes it afterwards.
import { execFile } from "node:child_process";
import { randomUUID } from "node:crypto";
import { promisify } from "node:util";
import type { WindmillBackendSettings } from "../../../../core/windmillBackendSettings";
import type { EvalCaseProfile } from "../../../../core/types";
import {
  syncedResourceTypes,
  type BenchmarkDatatableSeed,
  type BenchmarkWorkspaceRunnables,
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
const RESOURCE_TYPE_INDEX_WAIT_MS = 120_000;
const WORKSPACE_SLOT_WAIT_MS = 30 * 60_000;
/** The eval backend's own database, where a fixture user that is not the admin is created. */
const DATABASE_URL = process.env.WMILL_AI_EVAL_DATABASE_URL;

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
  let userToken: string | undefined;
  const dispose = async () => {
    activeWorkspaces.delete(id);
    if (input.user && DATABASE_URL) {
      await psql(`DELETE FROM token WHERE label = ${literal(tokenLabel(id))}`).catch(
        () => undefined,
      );
    }
    const workers = (input.fixtures.workers ?? []).map((w) => literal(w.worker));
    if (workers.length) {
      await psql(`DELETE FROM worker_ping WHERE worker IN (${workers.join(", ")})`).catch(
        () => undefined,
      );
    }
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
    userToken = await seedUser(api, id, input.user, input.profile);
    datatableDb = await seed(api, id, input.fixtures);
    activeWorkspaces.set(id, userToken ?? adminToken);
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
    // CE cannot create users through the API, so the user and a token of theirs are written
    // straight into the instance database, as the backend's own tests do. The workspace
    // membership still goes through the API.
    const email = `${username}@ai-evals.windmill.dev`;
    token = `aievals${randomUUID().replace(/-/g, "")}`;
    await psql(
      `INSERT INTO password (email, password_hash, login_type, super_admin, verified, name, username)
         VALUES (${literal(email)}, 'ai-evals-no-login', 'password', false, true, ${literal(username)},
                 ${literal(username)})
         ON CONFLICT (email) DO UPDATE SET username = EXCLUDED.username;
       INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin)
         VALUES (encode(sha256(${literal(token)}::bytea), 'hex'), ${literal(token.slice(0, 10))},
                 ${literal(token)}, ${literal(email)}, ${literal(tokenLabel(workspace))}, false);`,
    );
    await api("POST", "workspaces/add_user", {
      email,
      is_admin: user?.is_admin ?? false,
      operator: profile?.operator ?? false,
    });
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
  // A fixture that sets no `resourceTypes` stands for an instance synced with the hub,
  // which an eval backend usually is not; the types the instance already holds stay its own.
  const held = new Set<string>(await api("GET", "resources/type/listnames"));
  const resourceTypes = (fixtures.resourceTypes ?? syncedResourceTypes()).filter(
    (rt) => !held.has(rt.name),
  );
  for (const rt of resourceTypes) {
    await api("POST", "resources/type/create", {
      name: rt.name,
      schema: rt.schema ?? {},
      description: rt.description ?? "",
    });
  }
  // The resource type search reads an index the server rebuilds every
  // RESOURCE_TYPE_EMBEDDINGS_CHECK_INTERVAL_SECS: a case must not start before it holds them.
  const last = resourceTypes[resourceTypes.length - 1]?.name;
  for (const deadline = Date.now() + RESOURCE_TYPE_INDEX_WAIT_MS; last; ) {
    const found: { name: string }[] = await api(
      "GET",
      `embeddings/query_resource_types?text=${encodeURIComponent(last)}&limit=10`,
    );
    if (found.some((rt) => rt.name === last)) break;
    if (Date.now() > deadline) {
      throw new Error(`the resource type search never found the seeded "${last}"`);
    }
    await new Promise((resolve) => setTimeout(resolve, 2_000));
  }
  for (const group of fixtures.groups ?? []) {
    await api("POST", "groups/create", { name: group.name, summary: group.summary });
    // A fixture names members who are not users of the instance; a membership row needs none.
    for (const member of group.members ?? []) {
      await psql(
        `INSERT INTO usr_to_group (workspace_id, usr, group_)
           VALUES (${literal(workspace)}, ${literal(member)}, ${literal(group.name)})`,
        "This fixture seeds group members",
      );
    }
  }
  for (const folder of fixtures.folders ?? []) {
    await api("POST", "folders/create", {
      name: folder.name,
      summary: folder.summary,
      owners: folder.owners ?? [],
      extra_perms: folder.extra_perms ?? {},
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
    await seedJob(api, workspace, job);
  }
  for (const schedule of fixtures.schedules ?? []) {
    await api("POST", "schedules/create", {
      path: schedule.path,
      schedule: schedule.schedule,
      timezone: "UTC",
      script_path: schedule.script_path,
      is_flow: schedule.is_flow ?? false,
      args: {},
      enabled: schedule.enabled ?? true,
      summary: schedule.summary,
    });
  }
  await seedWorkers(fixtures.workers ?? []);
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
  workspace: string,
  job: NonNullable<BenchmarkWorkspaceRunnables["jobs"]>[number],
): Promise<void> {
  const args = job.args ?? {};
  const id = job.id ?? randomUUID();
  const state = job.state ?? "completed";
  if (state === "suspended") {
    // A flow parked on an approval step: its first step suspends, as the fixture flow's does.
    await api("POST", `jobs/run/f/${job.scriptPath}?job_id=${id}`, args);
    await waitFor(`run ${id} to suspend`, async () => {
      const run = await api("GET", `jobs_u/get/${id}?no_logs=true`);
      return (run.flow_status?.modules ?? []).some(
        (module: { type?: string }) => module.type === "WaitingForEvents",
      );
    });
  } else if (state === "queued") {
    // Stays queued only on a tag no worker of the eval backend serves, as the fixture's does.
    await api("POST", `jobs/run/preview?job_id=${id}`, {
      content: "export async function main() {}",
      language: "bun",
      path: job.scriptPath,
      args,
      tag: job.tag,
    });
  } else if (state === "completed") {
    const content = [
      `export async function main(${Object.keys(args).join(", ")}) {`,
      `  console.log(${JSON.stringify(job.logs ?? "")});`,
      job.success === false
        ? `  throw new Error(${JSON.stringify(lastLine(job.logs) ?? "Job failed")});`
        : `  return ${JSON.stringify(job.result ?? null)};`,
      `}`,
    ].join("\n");
    await api("POST", `jobs/run_wait_result/preview?job_id=${id}`, {
      content,
      language: "bun",
      path: job.scriptPath,
      args,
    }).catch((error) => {
      // A failing fixture run answers with its error, which is the point.
      if (job.success !== false) throw error;
    });
  } else {
    throw new Error(`A ${state} fixture run cannot be seeded on a real backend.`);
  }
  // Who ran it, when and where cannot be asked of the API: the run is rewritten to match.
  if (
    job.createdBy === undefined &&
    job.createdMinutesAgo === undefined &&
    job.durationMs === undefined &&
    job.tag === undefined &&
    job.schedulePath === undefined &&
    job.label === undefined
  ) {
    return;
  }
  const createdAt =
    job.createdMinutesAgo === undefined
      ? "created_at"
      : `now() - interval '${Number(job.createdMinutesAgo)} minutes'`;
  await psql(
    `UPDATE v2_job SET created_at = ${createdAt},
         created_by = COALESCE(${literal(job.createdBy)}, created_by),
         tag = COALESCE(${literal(job.tag)}, tag),
         labels = COALESCE(${job.label === undefined ? "NULL" : `ARRAY[${literal(job.label)}]`}, labels)
         ${job.schedulePath ? `, trigger = ${literal(job.schedulePath)}, trigger_kind = 'schedule'` : ""}
       WHERE id = ${literal(id)} AND workspace_id = ${literal(workspace)};
     UPDATE v2_job_queue q SET created_at = j.created_at, scheduled_for = j.created_at
       FROM v2_job j WHERE q.id = j.id AND q.id = ${literal(id)};
     UPDATE v2_job_completed c SET started_at = j.created_at,
         duration_ms = COALESCE(${literal(job.durationMs)}, c.duration_ms),
         completed_at = j.created_at + make_interval(secs => COALESCE(${literal(job.durationMs)}, c.duration_ms) / 1000.0)
       FROM v2_job j WHERE c.id = j.id AND c.id = ${literal(id)};`,
    "This fixture seeds who ran a run, when or where",
  );
}

/** Fixture workers, listed beside the instance's own until the attempt ends. */
async function seedWorkers(
  workers: NonNullable<BenchmarkWorkspaceRunnables["workers"]>,
): Promise<void> {
  // ponytail: rows are instance-wide, so concurrent attempts of one case share them.
  for (const worker of workers) {
    await psql(
      `INSERT INTO worker_ping (worker, worker_instance, ping_at, started_at, ip, jobs_executed,
                                custom_tags, worker_group)
         VALUES (${literal(worker.worker)}, ${literal(worker.worker)},
                 now() - interval '${Number(worker.lastPingSecondsAgo ?? 0)} seconds', now(), 'ai-evals',
                 ${Number(worker.jobs_executed ?? 0)},
                 ARRAY[${(worker.custom_tags ?? []).map(literal).join(", ")}]::varchar[],
                 ${literal(worker.worker_group)})
         ON CONFLICT (worker) DO UPDATE SET ping_at = EXCLUDED.ping_at`,
      "This fixture seeds workers",
    );
  }
}

async function waitFor(what: string, done: () => Promise<boolean>): Promise<void> {
  for (const deadline = Date.now() + 60_000; !(await done()); ) {
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
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
        },
      },
    },
  });
  // A new datatable keeps its schema as migrations; a fixture says when this one does not.
  if (!seed.migrations_enabled) {
    await api("POST", `workspaces/disable_datatable_migrations/${seed.datatable_name}`);
  }
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

/** Each attempt's user token carries its workspace, so the attempt revokes exactly its own. */
function tokenLabel(workspace: string): string {
  return `ai-evals ${workspace}`;
}

async function psql(
  sql: string,
  why = "This fixture's user is not the admin",
): Promise<void> {
  if (!DATABASE_URL) {
    throw new Error(
      `${why}: set WMILL_AI_EVAL_DATABASE_URL to the eval backend's database URL so the harness can write it.`,
    );
  }
  await promisify(execFile)("psql", [DATABASE_URL, "-v", "ON_ERROR_STOP=1", "-qc", sql]);
}
