/**
 * Shared deploy logic for workspace fork/merge operations.
 *
 * Used by both the CLI (`wmill workspace merge`) and the frontend
 * (`CompareWorkspaces.svelte`). The caller provides a {@link DeployProvider}
 * that wraps the concrete API client (class-based for the frontend,
 * standalone functions for the CLI).
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type DeployKind =
  | "script"
  | "flow"
  | "app"
  | "raw_app"
  | "resource"
  | "variable"
  | "resource_type"
  | "folder"
  | "schedule"
  | "http_trigger"
  | "websocket_trigger"
  | "kafka_trigger"
  | "nats_trigger"
  | "postgres_trigger"
  | "mqtt_trigger"
  | "sqs_trigger"
  | "gcp_trigger"
  | "azure_trigger"
  | "email_trigger";

export const TRIGGER_KINDS = [
  "http_trigger",
  "websocket_trigger",
  "kafka_trigger",
  "nats_trigger",
  "postgres_trigger",
  "mqtt_trigger",
  "sqs_trigger",
  "gcp_trigger",
  "azure_trigger",
  "email_trigger",
] as const satisfies readonly DeployKind[];

export type TriggerDeployKind = (typeof TRIGGER_KINDS)[number];

export function isTriggerKind(kind: string): kind is TriggerDeployKind {
  return (TRIGGER_KINDS as readonly string[]).includes(kind);
}

/** True for any kind that deploys via a trigger or schedule API. */
export function isTriggerOrScheduleKind(
  kind: string
): kind is TriggerDeployKind | "schedule" {
  return kind === "schedule" || isTriggerKind(kind);
}

export interface DeployResult {
  success: boolean;
  error?: string;
}

/**
 * Abstraction over the generated API client.
 * Both the frontend (class-based services) and the CLI (standalone functions)
 * can satisfy this interface with a thin adapter.
 */
export interface DeployProvider {
  // Existence checks
  existsFlowByPath(p: { workspace: string; path: string }): Promise<boolean>;
  existsScriptByPath(p: { workspace: string; path: string }): Promise<boolean>;
  existsApp(p: { workspace: string; path: string }): Promise<boolean>;
  existsVariable(p: { workspace: string; path: string }): Promise<boolean>;
  existsResource(p: { workspace: string; path: string }): Promise<boolean>;
  existsResourceType(p: { workspace: string; path: string }): Promise<boolean>;
  existsFolder(p: { workspace: string; name: string }): Promise<boolean>;
  // Flows
  getFlowByPath(p: { workspace: string; path: string }): Promise<any>;
  createFlow(p: { workspace: string; requestBody: any }): Promise<any>;
  updateFlow(p: {
    workspace: string;
    path: string;
    requestBody: any;
  }): Promise<any>;
  archiveFlowByPath(p: {
    workspace: string;
    path: string;
    requestBody: any;
  }): Promise<any>;
  // Scripts
  getScriptByPath(p: { workspace: string; path: string }): Promise<any>;
  createScript(p: { workspace: string; requestBody: any }): Promise<any>;
  archiveScriptByPath(p: {
    workspace: string;
    path: string;
  }): Promise<any>;
  // Apps
  getAppByPath(p: { workspace: string; path: string }): Promise<any>;
  createApp(p: { workspace: string; requestBody: any }): Promise<any>;
  updateApp(p: {
    workspace: string;
    path: string;
    requestBody: any;
  }): Promise<any>;
  createAppRaw(p: { workspace: string; formData: any }): Promise<any>;
  updateAppRaw(p: {
    workspace: string;
    path: string;
    formData: any;
  }): Promise<any>;
  getPublicSecretOfLatestVersionOfApp(p: {
    workspace: string;
    path: string;
  }): Promise<any>;
  getRawAppData(p: {
    secretWithExtension: string;
    workspace: string;
  }): Promise<any>;
  deleteApp(p: { workspace: string; path: string }): Promise<any>;
  // Variables
  getVariable(p: {
    workspace: string;
    path: string;
    decryptSecret?: boolean;
  }): Promise<any>;
  createVariable(p: { workspace: string; requestBody: any }): Promise<any>;
  updateVariable(p: {
    workspace: string;
    path: string;
    requestBody: any;
    alreadyEncrypted?: boolean;
  }): Promise<any>;
  deleteVariable(p: { workspace: string; path: string }): Promise<any>;
  // Resources
  getResource(p: { workspace: string; path: string }): Promise<any>;
  createResource(p: { workspace: string; requestBody: any }): Promise<any>;
  updateResource(p: {
    workspace: string;
    path: string;
    requestBody: any;
  }): Promise<any>;
  deleteResource(p: { workspace: string; path: string }): Promise<any>;
  // Resource types
  getResourceType(p: { workspace: string; path: string }): Promise<any>;
  createResourceType(p: { workspace: string; requestBody: any }): Promise<any>;
  updateResourceType(p: {
    workspace: string;
    path: string;
    requestBody: any;
  }): Promise<any>;
  deleteResourceType(p: { workspace: string; path: string }): Promise<any>;
  // Folders
  getFolder(p: { workspace: string; name: string }): Promise<any>;
  createFolder(p: { workspace: string; requestBody: any }): Promise<any>;
  updateFolder(p: {
    workspace: string;
    name: string;
    requestBody: any;
  }): Promise<any>;
  deleteFolder(p: { workspace: string; name: string }): Promise<any>;
  // Triggers — per-kind dispatch is delegated to the implementor so the shared
  // module doesn't need to know about each of the 9 trigger services.
  existsTriggerByKind(
    kind: TriggerDeployKind,
    p: { workspace: string; path: string }
  ): Promise<boolean>;
  /**
   * Get a trigger as it should be sent to create/update. Includes any kind-specific
   * transforms the implementor wants to apply before deploy (e.g. wiping GCP
   * subscription_id so the target workspace can create its own subscription).
   *
   * Operational-state handling (strip `mode`/`enabled` on update, pass through
   * on create) is applied by `deployItem` at the dispatch layer — implementors
   * should return the full row including `mode`/`enabled` and let the shared
   * dispatch decide.
   */
  getTriggerForDeploy(
    kind: TriggerDeployKind,
    p: { workspace: string; path: string; onBehalfOf?: string }
  ): Promise<any>;
  createTriggerByKind(
    kind: TriggerDeployKind,
    p: { workspace: string; requestBody: any }
  ): Promise<any>;
  updateTriggerByKind(
    kind: TriggerDeployKind,
    p: { workspace: string; path: string; requestBody: any }
  ): Promise<any>;
  deleteTriggerByKind(
    kind: TriggerDeployKind,
    p: { workspace: string; path: string }
  ): Promise<any>;
  /** Stripped trigger row used for the diff drawer (config fields only). */
  getTriggerValue(
    kind: TriggerDeployKind,
    p: { workspace: string; path: string }
  ): Promise<unknown>;
  /** Returns the trigger's `permissioned_as` for `--preserve-on-behalf-of`. */
  getTriggerPermissionedAs(
    kind: TriggerDeployKind,
    p: { workspace: string; path: string }
  ): Promise<string | undefined>;
  // Schedules
  existsSchedule(p: { workspace: string; path: string }): Promise<boolean>;
  getSchedule(p: { workspace: string; path: string }): Promise<any>;
  createSchedule(p: { workspace: string; requestBody: any }): Promise<any>;
  updateSchedule(p: {
    workspace: string;
    path: string;
    requestBody: any;
  }): Promise<any>;
  deleteSchedule(p: { workspace: string; path: string }): Promise<any>;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Folder diff paths carry the `f/` prefix; folder API endpoints expect just the name. */
export function folderName(path: string): string {
  return path.replace(/^f\//, "");
}

/**
 * Strip operational state (`mode`, `enabled`) from a trigger/schedule payload
 * when deploying as an update; pass through unchanged on create.
 *
 * On update the strip lets the backend preserve the target row's existing
 * state via `is_mode_unspecified()` (triggers) or via `EditSchedule` lacking
 * `enabled` (schedules). On create there's no target state to preserve, so
 * the source's flag is sent through — fork-only items land with whatever
 * state the fork creator chose, and missing flags fall back to the backend
 * default of enabled (`BaseTriggerData::mode()` / schedule insert default).
 */
export function stripOperationalStateOnUpdate<T extends Record<string, any>>(
  payload: T,
  alreadyExists: boolean
): T {
  if (!alreadyExists) return payload;
  const { mode: _mode, enabled: _enabled, ...rest } = payload;
  return rest as T;
}

function getSubModules(flowModule: any): any[][] {
  const type = flowModule?.value?.type;
  if (type === "forloopflow" || type === "whileloopflow") {
    return [flowModule.value.modules ?? []];
  } else if (type === "branchall") {
    return (flowModule.value.branches ?? []).map(
      (branch: any) => branch.modules ?? []
    );
  } else if (type === "branchone") {
    return [
      ...(flowModule.value.branches ?? []).map((b: any) => b.modules ?? []),
      flowModule.value.default ?? [],
    ];
  } else if (type === "aiagent") {
    if (flowModule.value.tools) {
      return [
        flowModule.value.tools
          .filter(
            (t: any) =>
              t.value?.type === "script" || t.value?.type === "flow"
          )
          .map((t: any) => ({
            id: t.id,
            value: t.value,
            summary: t.summary,
          })),
      ];
    }
  }
  return [];
}

function getAllSubmodules(flowModule: any): any[] {
  return getSubModules(flowModule)
    .map((modules) => modules.flatMap((m: any) => [m, ...getAllSubmodules(m)]))
    .flat();
}

/** Recursively collect all modules from a flow definition, including the failure module. */
export function getAllModules(
  flowModules: any[],
  failureModule?: any
): any[] {
  return [
    ...flowModules,
    ...flowModules.flatMap((x) => getAllSubmodules(x)),
    ...(failureModule ? [failureModule] : []),
  ];
}

function toError(e: unknown): string {
  const err = e as { body?: string; message?: string };
  return err.body || err.message || String(e);
}

// ---------------------------------------------------------------------------
// checkItemExists
// ---------------------------------------------------------------------------

export async function checkItemExists(
  provider: DeployProvider,
  kind: DeployKind,
  path: string,
  workspace: string
): Promise<boolean> {
  if (kind === "flow") {
    return provider.existsFlowByPath({ workspace, path });
  } else if (kind === "script") {
    return provider.existsScriptByPath({ workspace, path });
  } else if (kind === "app" || kind === "raw_app") {
    return provider.existsApp({ workspace, path });
  } else if (kind === "variable") {
    return provider.existsVariable({ workspace, path });
  } else if (kind === "resource") {
    return provider.existsResource({ workspace, path });
  } else if (kind === "resource_type") {
    return provider.existsResourceType({ workspace, path });
  } else if (kind === "folder") {
    return provider.existsFolder({ workspace, name: folderName(path) });
  } else if (kind === "schedule") {
    return provider.existsSchedule({ workspace, path });
  } else if (isTriggerKind(kind)) {
    return provider.existsTriggerByKind(kind, { workspace, path });
  }
  throw new Error(`Unknown kind: ${kind}`);
}

// ---------------------------------------------------------------------------
// deployItem
// ---------------------------------------------------------------------------

export async function deployItem(
  provider: DeployProvider,
  kind: DeployKind,
  path: string,
  workspaceFrom: string,
  workspaceTo: string,
  onBehalfOf?: string
): Promise<DeployResult> {
  const preserveOnBehalfOf = onBehalfOf !== undefined;

  try {
    const alreadyExists = await checkItemExists(
      provider,
      kind,
      path,
      workspaceTo
    );

    if (kind === "flow") {
      const flow = await provider.getFlowByPath({
        workspace: workspaceFrom,
        path,
      });
      // Clear inline script hashes so the target workspace resolves by path
      getAllModules(
        flow.value?.modules ?? [],
        flow.value?.failure_module
      ).forEach((x: any) => {
        if (x.value?.type === "script" && x.value.hash != undefined) {
          x.value.hash = undefined;
        }
      });
      if (alreadyExists) {
        await provider.updateFlow({
          workspace: workspaceTo,
          path,
          requestBody: {
            ...flow,
            preserve_on_behalf_of: preserveOnBehalfOf,
            on_behalf_of_email: onBehalfOf,
          },
        });
      } else {
        await provider.createFlow({
          workspace: workspaceTo,
          requestBody: {
            ...flow,
            preserve_on_behalf_of: preserveOnBehalfOf,
            on_behalf_of_email: onBehalfOf,
          },
        });
      }
    } else if (kind === "script") {
      const script = await provider.getScriptByPath({
        workspace: workspaceFrom,
        path,
      });
      let parentHash: string | undefined;
      if (alreadyExists) {
        const existing = await provider.getScriptByPath({
          workspace: workspaceTo,
          path,
        });
        parentHash = existing.hash;
      }
      await provider.createScript({
        workspace: workspaceTo,
        requestBody: {
          ...script,
          lock: script.lock,
          parent_hash: parentHash,
          preserve_on_behalf_of: preserveOnBehalfOf,
          on_behalf_of_email: onBehalfOf,
        },
      });
    } else if (kind === "app" || kind === "raw_app") {
      const app = await provider.getAppByPath({
        workspace: workspaceFrom,
        path,
      });
      if (alreadyExists) {
        if (app.raw_app) {
          const secret = await provider.getPublicSecretOfLatestVersionOfApp({
            workspace: workspaceFrom,
            path: app.path,
          });
          const js = await provider.getRawAppData({
            secretWithExtension: `${secret}.js`,
            workspace: workspaceFrom,
          });
          const css = await provider.getRawAppData({
            secretWithExtension: `${secret}.css`,
            workspace: workspaceFrom,
          });
          await provider.updateAppRaw({
            workspace: workspaceTo,
            path,
            formData: {
              app: { ...app, preserve_on_behalf_of: preserveOnBehalfOf },
              css,
              js,
            },
          });
        } else {
          await provider.updateApp({
            workspace: workspaceTo,
            path,
            requestBody: {
              ...app,
              preserve_on_behalf_of: preserveOnBehalfOf,
            },
          });
        }
      } else {
        if (app.raw_app) {
          const secret = await provider.getPublicSecretOfLatestVersionOfApp({
            workspace: workspaceFrom,
            path: app.path,
          });
          const js = await provider.getRawAppData({
            secretWithExtension: `${secret}.js`,
            workspace: workspaceFrom,
          });
          const css = await provider.getRawAppData({
            secretWithExtension: `${secret}.css`,
            workspace: workspaceFrom,
          });
          await provider.createAppRaw({
            workspace: workspaceTo,
            formData: {
              app: { ...app, preserve_on_behalf_of: preserveOnBehalfOf },
              css,
              js,
            },
          });
        } else {
          await provider.createApp({
            workspace: workspaceTo,
            requestBody: {
              ...app,
              preserve_on_behalf_of: preserveOnBehalfOf,
            },
          });
        }
      }
    } else if (kind === "variable") {
      const variable = await provider.getVariable({
        workspace: workspaceFrom,
        path,
        decryptSecret: true,
      });
      if (alreadyExists) {
        await provider.updateVariable({
          workspace: workspaceTo,
          path,
          requestBody: {
            path,
            value: variable.value ?? "",
            is_secret: variable.is_secret,
            description: variable.description ?? "",
          },
          alreadyEncrypted: false,
        });
      } else {
        await provider.createVariable({
          workspace: workspaceTo,
          requestBody: {
            path,
            value: variable.value ?? "",
            is_secret: variable.is_secret,
            description: variable.description ?? "",
          },
        });
      }
    } else if (kind === "resource") {
      const resource = await provider.getResource({
        workspace: workspaceFrom,
        path,
      });
      if (alreadyExists) {
        await provider.updateResource({
          workspace: workspaceTo,
          path,
          requestBody: {
            path,
            value: resource.value ?? "",
            description: resource.description ?? "",
          },
        });
      } else {
        await provider.createResource({
          workspace: workspaceTo,
          requestBody: {
            path,
            value: resource.value ?? "",
            resource_type: resource.resource_type,
            description: resource.description ?? "",
          },
        });
      }
    } else if (kind === "resource_type") {
      const rt = await provider.getResourceType({
        workspace: workspaceFrom,
        path,
      });
      if (alreadyExists) {
        await provider.updateResourceType({
          workspace: workspaceTo,
          path,
          requestBody: {
            schema: rt.schema,
            description: rt.description ?? "",
          },
        });
      } else {
        await provider.createResourceType({
          workspace: workspaceTo,
          requestBody: {
            name: rt.name,
            schema: rt.schema,
            description: rt.description ?? "",
          },
        });
      }
    } else if (kind === "folder") {
      const name = folderName(path);
      const folder = await provider.getFolder({
        workspace: workspaceFrom,
        name,
      });
      if (alreadyExists) {
        await provider.updateFolder({
          workspace: workspaceTo,
          name,
          requestBody: {
            owners: folder.owners,
            extra_perms: folder.extra_perms,
            summary: folder.summary ?? undefined,
          },
        });
      } else {
        await provider.createFolder({
          workspace: workspaceTo,
          requestBody: {
            name,
            owners: folder.owners,
            extra_perms: folder.extra_perms,
            summary: folder.summary ?? undefined,
          },
        });
      }
    } else if (kind === "schedule") {
      const schedule = await provider.getSchedule({
        workspace: workspaceFrom,
        path,
      });
      // Operational-state handling — same shape as triggers below:
      //   - Update: strip `enabled` so the target's existing state is preserved
      //     (`EditSchedule` lacks `enabled` server-side, so this is also
      //     enforced by the type, but stripping keeps the intent explicit).
      //   - Create: pass `enabled` through so a fork-only schedule lands with
      //     the state the fork creator chose.
      const baseBody = stripOperationalStateOnUpdate(schedule, alreadyExists);
      const requestBody = {
        ...baseBody,
        permissioned_as: onBehalfOf,
        preserve_permissioned_as: preserveOnBehalfOf,
      };
      if (alreadyExists) {
        await provider.updateSchedule({
          workspace: workspaceTo,
          path,
          requestBody,
        });
      } else {
        await provider.createSchedule({
          workspace: workspaceTo,
          requestBody,
        });
      }
    } else if (isTriggerKind(kind)) {
      const triggerBody = await provider.getTriggerForDeploy(kind, {
        workspace: workspaceFrom,
        path,
        onBehalfOf,
      });
      // Strip operational state on update; pass through on create. The
      // implementor's `getTriggerForDeploy` is responsible only for kind-specific
      // transforms (e.g. GCP subscription_id wipe) — the operational-state
      // strip lives here so it stays consistent with the schedule branch and
      // with the legacy `kind === 'trigger'` path in the frontend.
      const requestBody = stripOperationalStateOnUpdate(triggerBody, alreadyExists);
      if (alreadyExists) {
        await provider.updateTriggerByKind(kind, {
          workspace: workspaceTo,
          path,
          requestBody,
        });
      } else {
        await provider.createTriggerByKind(kind, {
          workspace: workspaceTo,
          requestBody,
        });
      }
    } else {
      throw new Error(`Unknown kind: ${kind}`);
    }

    return { success: true };
  } catch (e: unknown) {
    return { success: false, error: toError(e) };
  }
}

// ---------------------------------------------------------------------------
// deleteItemInWorkspace
// ---------------------------------------------------------------------------

/**
 * Delete/archive an item in a workspace.
 * Scripts and flows are archived (reversible). Other types are deleted.
 */
export async function deleteItemInWorkspace(
  provider: DeployProvider,
  kind: DeployKind,
  path: string,
  workspace: string
): Promise<DeployResult> {
  try {
    if (kind === "script") {
      await provider.archiveScriptByPath({ workspace, path });
    } else if (kind === "flow") {
      await provider.archiveFlowByPath({
        workspace,
        path,
        requestBody: { archived: true },
      });
    } else if (kind === "app" || kind === "raw_app") {
      await provider.deleteApp({ workspace, path });
    } else if (kind === "variable") {
      await provider.deleteVariable({ workspace, path });
    } else if (kind === "resource") {
      await provider.deleteResource({ workspace, path });
    } else if (kind === "resource_type") {
      await provider.deleteResourceType({ workspace, path });
    } else if (kind === "folder") {
      await provider.deleteFolder({ workspace, name: folderName(path) });
    } else if (kind === "schedule") {
      await provider.deleteSchedule({ workspace, path });
    } else if (isTriggerKind(kind)) {
      await provider.deleteTriggerByKind(kind, { workspace, path });
    } else {
      throw new Error(`Deletion not supported for kind: ${kind}`);
    }
    return { success: true };
  } catch (e: unknown) {
    return { success: false, error: toError(e) };
  }
}

// ---------------------------------------------------------------------------
// getOnBehalfOf
// ---------------------------------------------------------------------------

/**
 * Get the value of an item for diff comparison.
 * Returns a normalized representation suitable for JSON comparison.
 */
export async function getItemValue(
  provider: DeployProvider,
  kind: DeployKind,
  path: string,
  workspace: string
): Promise<unknown> {
  try {
    if (kind === "flow") {
      const flow = await provider.getFlowByPath({ workspace, path });
      getAllModules(flow.value?.modules ?? [], flow.value?.failure_module).forEach(
        (x: any) => {
          if (x.value?.type === "script" && x.value.hash != undefined) {
            x.value.hash = undefined;
          }
        }
      );
      return {
        summary: flow.summary,
        description: flow.description,
        value: flow.value,
      };
    } else if (kind === "script") {
      const script = await provider.getScriptByPath({ workspace, path });
      return {
        content: script.content,
        lock: script.lock,
        schema: script.schema,
        summary: script.summary,
        language: script.language,
      };
    } else if (kind === "app" || kind === "raw_app") {
      return await provider.getAppByPath({ workspace, path });
    } else if (kind === "variable") {
      const variable = await provider.getVariable({
        workspace,
        path,
        decryptSecret: true,
      });
      return variable.value;
    } else if (kind === "resource") {
      const resource = await provider.getResource({ workspace, path });
      return resource.value;
    } else if (kind === "resource_type") {
      const rt = await provider.getResourceType({ workspace, path });
      return rt.schema;
    } else if (kind === "folder") {
      const folder = await provider.getFolder({
        workspace,
        name: folderName(path),
      });
      return {
        name: folder.name,
        owners: folder.owners,
        extra_perms: folder.extra_perms,
        summary: folder.summary,
      };
    } else if (kind === "schedule") {
      // Mirror the runtime-fields-ignore set used server-side so the diff drawer
      // matches the merge UI's "did config change?" semantics.
      const schedule = await provider.getSchedule({ workspace, path });
      return stripTriggerOrScheduleRuntimeFields(schedule);
    } else if (isTriggerKind(kind)) {
      const trigger = await provider.getTriggerValue(kind, { workspace, path });
      return stripTriggerOrScheduleRuntimeFields(trigger);
    }
  } catch {
    // Item may not exist
  }
  return {};
}

/**
 * Strip runtime fields from a trigger or schedule row so the diff drawer
 * shows only config differences. Mirrors the backend's
 * `TRIGGER_COMPARE_IGNORE` constant.
 */
function stripTriggerOrScheduleRuntimeFields(row: unknown): unknown {
  if (!row || typeof row !== "object") return row;
  const ignore = new Set([
    "workspace_id",
    "edited_by",
    "edited_at",
    "email",
    "error",
    "enabled",
    "mode",
    "server_id",
    "last_server_ping",
    "extra_perms",
    "permissioned_as",
    // Server-managed (kept in sync with backend `TRIGGER_COMPARE_IGNORE`).
    "subscription_id",
    "push_auth_config",
  ]);
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(row as Record<string, unknown>)) {
    if (!ignore.has(k)) out[k] = v;
  }
  return out;
}

/**
 * Fetch the on_behalf_of value for a deployable item.
 * Returns an email for flows/scripts/apps, or undefined if not applicable.
 */
export async function getOnBehalfOf(
  provider: DeployProvider,
  kind: DeployKind,
  path: string,
  workspace: string
): Promise<string | undefined> {
  try {
    if (kind === "flow") {
      const flow = await provider.getFlowByPath({ workspace, path });
      return flow.on_behalf_of_email;
    } else if (kind === "script") {
      const script = await provider.getScriptByPath({ workspace, path });
      return script.on_behalf_of_email;
    } else if (kind === "app" || kind === "raw_app") {
      const app = await provider.getAppByPath({ workspace, path });
      return app.policy?.on_behalf_of_email;
    } else if (kind === "schedule") {
      const schedule = await provider.getSchedule({ workspace, path });
      return schedule.permissioned_as;
    } else if (isTriggerKind(kind)) {
      return await provider.getTriggerPermissionedAs(kind, { workspace, path });
    }
  } catch {
    // Item may not exist
  }
  return undefined;
}
