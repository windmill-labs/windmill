/**
 * The merge as the CLI ran it before the server did it: each item read from one workspace and
 * written to the other through the ordinary item routes. Used only against a server without
 * `deploy_from_workspace`; delete once no supported server predates it.
 */
import * as wmill from "../../../gen/services.gen.ts";
import { OpenAPI } from "../../../gen/core/OpenAPI.ts";
import {
  deleteItemInWorkspace,
  deployItem,
  type DeployKind,
  type DeployProvider,
  type TriggerDeployKind,
} from "../../../windmill-utils-internal/src/deploy.ts";


// ---------------------------------------------------------------------------
// Provider adapter — wraps CLI's standalone API functions
// ---------------------------------------------------------------------------

const provider: DeployProvider = {
  existsFlowByPath: wmill.existsFlowByPath,
  existsScriptByPath: wmill.existsScriptByPath,
  existsApp: wmill.existsApp,
  existsVariable: wmill.existsVariable,
  existsResource: wmill.existsResource,
  existsResourceType: wmill.existsResourceType,
  existsFolder: wmill.existsFolder,
  getFlowByPath: wmill.getFlowByPath,
  createFlow: wmill.createFlow,
  updateFlow: wmill.updateFlow,
  archiveFlowByPath: wmill.archiveFlowByPath,
  getScriptByPath: wmill.getScriptByPath,
  createScript: wmill.createScript,
  archiveScriptByPath: wmill.archiveScriptByPath,
  getAppByPath: wmill.getAppByPath,
  createApp: wmill.createApp,
  updateApp: wmill.updateApp,
  createAppRaw: wmill.createAppRaw,
  updateAppRaw: wmill.updateAppRaw,
  getPublicSecretOfLatestVersionOfApp:
    wmill.getPublicSecretOfLatestVersionOfApp,
  getRawAppData: wmill.getRawAppData,
  deleteApp: wmill.deleteApp,
  getVariable: wmill.getVariable,
  createVariable: wmill.createVariable,
  updateVariable: wmill.updateVariable,
  deleteVariable: wmill.deleteVariable,
  getResource: wmill.getResource,
  createResource: wmill.createResource,
  updateResource: wmill.updateResource,
  deleteResource: wmill.deleteResource,
  getResourceType: wmill.getResourceType,
  createResourceType: wmill.createResourceType,
  updateResourceType: wmill.updateResourceType,
  deleteResourceType: wmill.deleteResourceType,
  getFolder: wmill.getFolder,
  createFolder: wmill.createFolder,
  updateFolder: wmill.updateFolder,
  deleteFolder: wmill.deleteFolder,
  // Triggers — per-kind dispatch on the new TriggerDeployKind union.
  existsTriggerByKind: (kind, p) => triggerService(kind).exists(p),
  getTriggerForDeploy: async (kind, p) => {
    const trigger = await triggerService(kind).get({
      workspace: p.workspace,
      path: p.path,
    });
    return preparePayload(kind, trigger, p.onBehalfOf);
  },
  createTriggerByKind: (kind, p) =>
    triggerService(kind).create(p as Parameters<ReturnType<typeof triggerService>["create"]>[0]),
  updateTriggerByKind: (kind, p) =>
    triggerService(kind).update(p as Parameters<ReturnType<typeof triggerService>["update"]>[0]),
  deleteTriggerByKind: (kind, p) => triggerService(kind).delete(p),
  getTriggerValue: (kind, p) => triggerService(kind).get(p),
  getTriggerPermissionedAs: async (kind, p) => {
    const trigger = await triggerService(kind).get(p);
    return trigger?.permissioned_as;
  },
  // Schedules
  existsSchedule: wmill.existsSchedule,
  getSchedule: wmill.getSchedule,
  createSchedule: wmill.createSchedule,
  updateSchedule: wmill.updateSchedule,
  deleteSchedule: wmill.deleteSchedule,
  // Datatable migrations
  listDatatableMigrations: wmill.listDatatableMigrations,
  upsertDatatableMigration: wmill.upsertDatatableMigration,
  deleteDatatableMigration: wmill.deleteDatatableMigration,
};

/**
 * Per-kind trigger dispatch. Returns the standalone CLI service functions for
 * one of the nine deployable trigger kinds.
 */
function triggerService(kind: TriggerDeployKind) {
  switch (kind) {
    case "http_trigger":
      return {
        exists: wmill.existsHttpTrigger,
        get: wmill.getHttpTrigger,
        create: wmill.createHttpTrigger,
        update: wmill.updateHttpTrigger,
        delete: wmill.deleteHttpTrigger,
      };
    case "websocket_trigger":
      return {
        exists: wmill.existsWebsocketTrigger,
        get: wmill.getWebsocketTrigger,
        create: wmill.createWebsocketTrigger,
        update: wmill.updateWebsocketTrigger,
        delete: wmill.deleteWebsocketTrigger,
      };
    case "kafka_trigger":
      return {
        exists: wmill.existsKafkaTrigger,
        get: wmill.getKafkaTrigger,
        create: wmill.createKafkaTrigger,
        update: wmill.updateKafkaTrigger,
        delete: wmill.deleteKafkaTrigger,
      };
    case "nats_trigger":
      return {
        exists: wmill.existsNatsTrigger,
        get: wmill.getNatsTrigger,
        create: wmill.createNatsTrigger,
        update: wmill.updateNatsTrigger,
        delete: wmill.deleteNatsTrigger,
      };
    case "postgres_trigger":
      return {
        exists: wmill.existsPostgresTrigger,
        get: wmill.getPostgresTrigger,
        create: wmill.createPostgresTrigger,
        update: wmill.updatePostgresTrigger,
        delete: wmill.deletePostgresTrigger,
      };
    case "mqtt_trigger":
      return {
        exists: wmill.existsMqttTrigger,
        get: wmill.getMqttTrigger,
        create: wmill.createMqttTrigger,
        update: wmill.updateMqttTrigger,
        delete: wmill.deleteMqttTrigger,
      };
    case "sqs_trigger":
      return {
        exists: wmill.existsSqsTrigger,
        get: wmill.getSqsTrigger,
        create: wmill.createSqsTrigger,
        update: wmill.updateSqsTrigger,
        delete: wmill.deleteSqsTrigger,
      };
    case "gcp_trigger":
      return {
        exists: wmill.existsGcpTrigger,
        get: wmill.getGcpTrigger,
        create: wmill.createGcpTrigger,
        update: wmill.updateGcpTrigger,
        delete: wmill.deleteGcpTrigger,
      };
    case "azure_trigger":
      return {
        exists: wmill.existsAzureTrigger,
        get: wmill.getAzureTrigger,
        create: wmill.createAzureTrigger,
        update: wmill.updateAzureTrigger,
        delete: wmill.deleteAzureTrigger,
      };
    case "email_trigger":
      return {
        exists: wmill.existsEmailTrigger,
        get: wmill.getEmailTrigger,
        create: wmill.createEmailTrigger,
        update: wmill.updateEmailTrigger,
        delete: wmill.deleteEmailTrigger,
      };
  }
}

/**
 * Apply kind-specific transforms before sending a trigger to create/update.
 * Currently only GCP needs special handling (subscription_id wipe + base_endpoint
 * for push delivery). All kinds receive the on_behalf_of plumbing.
 *
 * Operational state (`mode`/`enabled`) is intentionally NOT stripped here —
 * the shared `deployItem` strips on update and passes through on create via
 * `stripOperationalStateOnUpdate`.
 */
function preparePayload(
  kind: TriggerDeployKind,
  trigger: any,
  onBehalfOf?: string
): any {
  const preserve = onBehalfOf !== undefined;
  const base: any = {
    ...trigger,
    permissioned_as: onBehalfOf,
    preserve_permissioned_as: preserve,
  };
  if (kind === "gcp_trigger") {
    // The target workspace creates its own pubsub subscription on deploy; the
    // source's subscription_id would be invalid. `audience` (push delivery
    // verification) is similarly target-specific — wipe so the backend can
    // recompute from `base_endpoint`. Mirrors `getTriggersDeployData` in
    // frontend/src/lib/utils_deployable.ts.
    base.subscription_id = "";
    base.subscription_mode = "create_update";
    if (base.delivery_config) {
      base.delivery_config = { ...base.delivery_config, audience: "" };
    }
    if (base.delivery_type === "push") {
      // The CLI's configured remote URL is shared by the parent and fork
      // workspaces (same instance), so it doubles as the push delivery
      // endpoint for either deploy direction. `OpenAPI.BASE` is
      // `<host>/api`; strip the `/api` suffix so the value matches what
      // the frontend's `getTriggersDeployData` and the GCP/Azure trigger
      // editors set (`${origin}${base}`). The backend composes the
      // Pub/Sub push URL as `${base_endpoint}/${path}`, so an extra
      // `/api` segment would point Pub/Sub at a route the backend
      // doesn't serve.
      base.base_endpoint = OpenAPI.BASE.replace(/\/api\/?$/, "");
    } else {
      base.base_endpoint = undefined;
    }
  }
  return base;
}


export function legacyDeploy(
  kind: DeployKind,
  path: string,
  workspaceFrom: string,
  workspaceTo: string,
  onBehalfOf?: string
) {
  return deployItem(provider, kind, path, workspaceFrom, workspaceTo, onBehalfOf);
}

export function legacyDelete(kind: DeployKind, path: string, workspace: string) {
  return deleteItemInWorkspace(provider, kind, path, workspace);
}

