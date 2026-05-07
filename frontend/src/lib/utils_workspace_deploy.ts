import {
	AppService,
	AzureTriggerService,
	EmailTriggerService,
	FlowService,
	FolderService,
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ResourceService,
	ScheduleService,
	ScriptService,
	SqsTriggerService,
	VariableService,
	WebsocketTriggerService
} from '$lib/gen'
import {
	existsTrigger,
	getTriggersDeployData,
	getTriggerPermissionedAs,
	getTriggerValue,
	stripOperationalState,
	type AdditionalInformation,
	type Kind
} from '$lib/utils_deployable'

import {
	deployItem as sharedDeployItem,
	deleteItemInWorkspace as sharedDeleteItem,
	checkItemExists as sharedCheckItemExists,
	getOnBehalfOf as sharedGetOnBehalfOf,
	getItemValue as sharedGetItemValue,
	type DeployProvider,
	type DeployKind,
	type DeployResult,
	type TriggerDeployKind
} from 'windmill-utils-internal'

export type { DeployResult, DeployKind, TriggerDeployKind }

// ---------------------------------------------------------------------------
// Provider adapter — wraps frontend's class-based services
// ---------------------------------------------------------------------------

/**
 * Map a shared `TriggerDeployKind` (e.g. `kafka_trigger`) to the per-kind
 * service class. Per-kind dispatch lives in the adapter so the shared
 * `deployItem` only needs to know "trigger" vs "not trigger".
 */
function triggerServiceFor(kind: TriggerDeployKind) {
	switch (kind) {
		case 'http_trigger':
			return {
				exists: HttpTriggerService.existsHttpTrigger,
				get: HttpTriggerService.getHttpTrigger,
				create: HttpTriggerService.createHttpTrigger,
				update: HttpTriggerService.updateHttpTrigger,
				delete: HttpTriggerService.deleteHttpTrigger
			}
		case 'websocket_trigger':
			return {
				exists: WebsocketTriggerService.existsWebsocketTrigger,
				get: WebsocketTriggerService.getWebsocketTrigger,
				create: WebsocketTriggerService.createWebsocketTrigger,
				update: WebsocketTriggerService.updateWebsocketTrigger,
				delete: WebsocketTriggerService.deleteWebsocketTrigger
			}
		case 'kafka_trigger':
			return {
				exists: KafkaTriggerService.existsKafkaTrigger,
				get: KafkaTriggerService.getKafkaTrigger,
				create: KafkaTriggerService.createKafkaTrigger,
				update: KafkaTriggerService.updateKafkaTrigger,
				delete: KafkaTriggerService.deleteKafkaTrigger
			}
		case 'nats_trigger':
			return {
				exists: NatsTriggerService.existsNatsTrigger,
				get: NatsTriggerService.getNatsTrigger,
				create: NatsTriggerService.createNatsTrigger,
				update: NatsTriggerService.updateNatsTrigger,
				delete: NatsTriggerService.deleteNatsTrigger
			}
		case 'postgres_trigger':
			return {
				exists: PostgresTriggerService.existsPostgresTrigger,
				get: PostgresTriggerService.getPostgresTrigger,
				create: PostgresTriggerService.createPostgresTrigger,
				update: PostgresTriggerService.updatePostgresTrigger,
				delete: PostgresTriggerService.deletePostgresTrigger
			}
		case 'mqtt_trigger':
			return {
				exists: MqttTriggerService.existsMqttTrigger,
				get: MqttTriggerService.getMqttTrigger,
				create: MqttTriggerService.createMqttTrigger,
				update: MqttTriggerService.updateMqttTrigger,
				delete: MqttTriggerService.deleteMqttTrigger
			}
		case 'sqs_trigger':
			return {
				exists: SqsTriggerService.existsSqsTrigger,
				get: SqsTriggerService.getSqsTrigger,
				create: SqsTriggerService.createSqsTrigger,
				update: SqsTriggerService.updateSqsTrigger,
				delete: SqsTriggerService.deleteSqsTrigger
			}
		case 'gcp_trigger':
			return {
				exists: GcpTriggerService.existsGcpTrigger,
				get: GcpTriggerService.getGcpTrigger,
				create: GcpTriggerService.createGcpTrigger,
				update: GcpTriggerService.updateGcpTrigger,
				delete: GcpTriggerService.deleteGcpTrigger
			}
		case 'azure_trigger':
			return {
				exists: AzureTriggerService.existsAzureTrigger,
				get: AzureTriggerService.getAzureTrigger,
				create: AzureTriggerService.createAzureTrigger,
				update: AzureTriggerService.updateAzureTrigger,
				delete: AzureTriggerService.deleteAzureTrigger
			}
		case 'email_trigger':
			return {
				exists: EmailTriggerService.existsEmailTrigger,
				get: EmailTriggerService.getEmailTrigger,
				create: EmailTriggerService.createEmailTrigger,
				update: EmailTriggerService.updateEmailTrigger,
				delete: EmailTriggerService.deleteEmailTrigger
			}
		default: {
			// Exhaustiveness guard: extending TriggerDeployKind without a case here
			// produces a compile error rather than a silent runtime failure.
			const _exhaustive: never = kind
			throw new Error(`Unhandled trigger kind: ${_exhaustive}`)
		}
	}
}

/**
 * Map the shared `TriggerDeployKind` to the legacy frontend `TriggerKind`
 * used by helpers in `utils_deployable.ts`.
 */
function legacyTriggerKind(kind: TriggerDeployKind) {
	const map = {
		http_trigger: 'routes',
		websocket_trigger: 'websockets',
		kafka_trigger: 'kafka',
		nats_trigger: 'nats',
		postgres_trigger: 'postgres',
		mqtt_trigger: 'mqtt',
		sqs_trigger: 'sqs',
		gcp_trigger: 'gcp',
		azure_trigger: 'azure',
		email_trigger: 'emails'
	} as const
	return map[kind]
}

function makeProvider(): DeployProvider {
	return {
		existsFlowByPath: (p) => FlowService.existsFlowByPath(p),
		existsScriptByPath: (p) => ScriptService.existsScriptByPath(p),
		existsApp: (p) => AppService.existsApp(p),
		existsVariable: (p) => VariableService.existsVariable(p),
		existsResource: (p) => ResourceService.existsResource(p),
		existsResourceType: (p) => ResourceService.existsResourceType(p),
		existsFolder: (p) => FolderService.existsFolder(p),
		getFlowByPath: (p) => FlowService.getFlowByPath(p),
		createFlow: (p) => FlowService.createFlow(p),
		updateFlow: (p) => FlowService.updateFlow(p),
		archiveFlowByPath: (p) => FlowService.archiveFlowByPath(p),
		getScriptByPath: (p) => ScriptService.getScriptByPath(p),
		createScript: (p) => ScriptService.createScript(p),
		archiveScriptByPath: (p) => ScriptService.archiveScriptByPath(p),
		getAppByPath: (p) => AppService.getAppByPath(p),
		createApp: (p) => AppService.createApp(p),
		updateApp: (p) => AppService.updateApp(p),
		createAppRaw: (p) => AppService.createAppRaw(p),
		updateAppRaw: (p) => AppService.updateAppRaw(p),
		getPublicSecretOfLatestVersionOfApp: (p) => AppService.getPublicSecretOfLatestVersionOfApp(p),
		getRawAppData: (p) => AppService.getRawAppData(p),
		deleteApp: (p) => AppService.deleteApp(p),
		getVariable: (p) => VariableService.getVariable(p),
		createVariable: (p) => VariableService.createVariable(p),
		updateVariable: (p) => VariableService.updateVariable(p),
		deleteVariable: (p) => VariableService.deleteVariable(p),
		getResource: (p) => ResourceService.getResource(p),
		createResource: (p) => ResourceService.createResource(p),
		updateResource: (p) => ResourceService.updateResource(p),
		deleteResource: (p) => ResourceService.deleteResource(p),
		getResourceType: (p) => ResourceService.getResourceType(p),
		createResourceType: (p) => ResourceService.createResourceType(p),
		updateResourceType: (p) => ResourceService.updateResourceType(p),
		deleteResourceType: (p) => ResourceService.deleteResourceType(p),
		getFolder: (p) => FolderService.getFolder(p),
		createFolder: (p) => FolderService.createFolder(p),
		updateFolder: (p) => FolderService.updateFolder(p),
		deleteFolder: (p) => FolderService.deleteFolder(p),
		// Triggers
		existsTriggerByKind: (kind, p) => triggerServiceFor(kind).exists(p),
		getTriggerForDeploy: async (kind, p) => {
			// Reuses the existing per-kind transform map (e.g. GCP wipes
			// subscription_id and computes base_endpoint from window.location).
			// Operational-state strip is applied by the shared `deployItem`
			// after this returns.
			const { data } = await getTriggersDeployData(
				legacyTriggerKind(kind),
				p.path,
				p.workspace,
				p.onBehalfOf
			)
			return data
		},
		createTriggerByKind: (kind, p) => triggerServiceFor(kind).create(p as any),
		updateTriggerByKind: (kind, p) => triggerServiceFor(kind).update(p as any),
		deleteTriggerByKind: (kind, p) => triggerServiceFor(kind).delete(p),
		getTriggerValue: (kind, p) => getTriggerValue(legacyTriggerKind(kind), p.path, p.workspace),
		getTriggerPermissionedAs: async (kind, p) => {
			const trigger = await triggerServiceFor(kind).get(p)
			return (trigger as any)?.permissioned_as
		},
		// Schedules
		existsSchedule: (p) => ScheduleService.existsSchedule(p),
		getSchedule: (p) => ScheduleService.getSchedule(p),
		createSchedule: (p) => ScheduleService.createSchedule(p),
		updateSchedule: (p) => ScheduleService.updateSchedule(p),
		deleteSchedule: (p) => ScheduleService.deleteSchedule(p)
	}
}

// ---------------------------------------------------------------------------
// Public API — thin wrappers over the shared dispatch
// ---------------------------------------------------------------------------

export interface DeployItemParams {
	kind: Kind
	path: string
	workspaceFrom: string
	workspaceTo: string
	/**
	 * Carries the trigger sub-kind for the legacy generic `kind: 'trigger'` path
	 * used by `DeployWorkspace.svelte`. Not needed for the new per-kind names
	 * (`http_trigger`, `kafka_trigger`, …) returned by the fork-merge compare API.
	 */
	additionalInformation?: AdditionalInformation
	/**
	 * The value to use for on_behalf_of when deploying.
	 * Format varies by item kind:
	 * - For flows/scripts/apps: an email address (on_behalf_of_email)
	 * - For triggers/schedules: permissioned_as format (u/username or g/group)
	 * If set, preserve_on_behalf_of / preserve_permissioned_as will be true.
	 * If undefined, the deploying user's identity is used.
	 */
	onBehalfOf?: string
}

/**
 * Deploy an item from one workspace to another. Handles every kind in the shared
 * `DeployKind` union plus the legacy generic `'trigger'` from `DeployWorkspace.svelte`,
 * which carries its sub-kind in `additionalInformation`.
 */
export async function deployItem(params: DeployItemParams): Promise<DeployResult> {
	const { kind, path, workspaceFrom, workspaceTo, additionalInformation, onBehalfOf } = params

	if (kind === 'trigger') {
		// Legacy path: `DeployWorkspace.svelte` doesn't know the per-kind trigger
		// name when building dependency graphs, so it passes `kind: 'trigger'` and
		// the actual sub-kind in `additionalInformation`. Translate to per-kind.
		if (!additionalInformation?.triggers) {
			return { success: false, error: 'Missing triggers kind' }
		}
		try {
			const alreadyExists = await existsTrigger(
				{ workspace: workspaceTo, path },
				additionalInformation.triggers.kind
			)
			const { data, createFn, updateFn } = await getTriggersDeployData(
				additionalInformation.triggers.kind,
				path,
				workspaceFrom,
				onBehalfOf
			)
			if (alreadyExists) {
				// Strip operational state so the update doesn't flip the target's
				// existing enabled/mode flag — preserved via `is_mode_unspecified()`
				// on the backend. Mirrors the shared `stripOperationalStateOnUpdate`
				// in the merge-deploy path.
				const stripped = stripOperationalState(data)
				await updateFn({ path, workspace: workspaceTo, requestBody: stripped } as any)
			} else {
				// Create — pass source's `mode`/`enabled` through so a new
				// trigger lands with the state the source workspace had.
				await createFn({ workspace: workspaceTo, requestBody: data } as any)
			}
			return { success: true }
		} catch (e: any) {
			return { success: false, error: e.body || e.message || String(e) }
		}
	}

	return sharedDeployItem(
		makeProvider(),
		kind as DeployKind,
		path,
		workspaceFrom,
		workspaceTo,
		onBehalfOf
	)
}

/**
 * Delete/archive an item in a workspace.
 * Used when deploying a deletion from one workspace to another.
 * Scripts and flows are archived (reversible). Other types are deleted.
 */
export async function deleteItemInWorkspace(
	kind: Kind,
	path: string,
	workspace: string
): Promise<DeployResult> {
	return sharedDeleteItem(makeProvider(), kind as DeployKind, path, workspace)
}

/**
 * Check if an item already exists in the target workspace.
 */
export async function checkItemExists(
	kind: Kind,
	path: string,
	workspace: string,
	additionalInformation?: AdditionalInformation
): Promise<boolean> {
	if (kind === 'trigger') {
		if (!additionalInformation?.triggers) {
			throw new Error('Missing triggers kind for legacy trigger deploy')
		}
		return existsTrigger({ workspace, path }, additionalInformation.triggers.kind)
	}
	return sharedCheckItemExists(makeProvider(), kind as DeployKind, path, workspace)
}

/**
 * Get the value of an item for diff comparison.
 */
export async function getItemValue(
	kind: Kind,
	path: string,
	workspace: string,
	additionalInformation?: AdditionalInformation
): Promise<unknown> {
	if (kind === 'trigger') {
		if (!additionalInformation?.triggers) return {}
		try {
			return await getTriggerValue(additionalInformation.triggers.kind, path, workspace)
		} catch {
			return {}
		}
	}
	return sharedGetItemValue(makeProvider(), kind as DeployKind, path, workspace)
}

/**
 * Get the on_behalf_of value for a deployable item.
 */
export async function getOnBehalfOf(
	kind: Kind,
	path: string,
	workspace: string,
	additionalInformation?: AdditionalInformation
): Promise<string | undefined> {
	if (kind === 'trigger' && additionalInformation?.triggers) {
		try {
			return await getTriggerPermissionedAs(additionalInformation.triggers.kind, path, workspace)
		} catch {
			return undefined
		}
	}
	return sharedGetOnBehalfOf(makeProvider(), kind as DeployKind, path, workspace)
}
