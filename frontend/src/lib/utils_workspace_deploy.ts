import {
	AppService,
	FlowService,
	FolderService,
	ResourceService,
	ScheduleService,
	ScriptService,
	VariableService
} from '$lib/gen'
import {
	existsTrigger,
	getTriggersDeployData,
	getTriggerPermissionedAs,
	getTriggerValue,
	type AdditionalInformation,
	type Kind
} from '$lib/utils_deployable'
import type { TriggerKind } from './components/triggers'

import {
	deployItem as sharedDeployItem,
	deleteItemInWorkspace as sharedDeleteItem,
	checkItemExists as sharedCheckItemExists,
	getOnBehalfOf as sharedGetOnBehalfOf,
	getItemValue as sharedGetItemValue,
	type DeployProvider,
	type DeployKind,
	type DeployResult
} from 'windmill-utils-internal'

export type { DeployResult }

// ---------------------------------------------------------------------------
// Provider adapter — wraps frontend's class-based services
// ---------------------------------------------------------------------------

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
		deleteFolder: (p) => FolderService.deleteFolder(p)
	}
}

// ---------------------------------------------------------------------------
// Public API — thin wrappers that add trigger handling (frontend-specific)
// ---------------------------------------------------------------------------

export interface DeployItemParams {
	kind: Kind
	path: string
	workspaceFrom: string
	workspaceTo: string
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
 * Deploy an item from one workspace to another.
 * Handles all item kinds: flow, script, app, variable, resource, resource_type, folder, trigger.
 */
export async function deployItem(params: DeployItemParams): Promise<DeployResult> {
	const { kind, path, workspaceFrom, workspaceTo, additionalInformation, onBehalfOf } = params

	// Triggers are frontend-specific (not in the shared module)
	if (kind === 'trigger') {
		if (!additionalInformation?.triggers) {
			return { success: false, error: 'Missing triggers kind' }
		}
		try {
			const alreadyExists = await checkItemExists(kind, path, workspaceTo, additionalInformation)
			const { data, createFn, updateFn } = await getTriggersDeployData(
				additionalInformation.triggers.kind,
				path,
				workspaceFrom,
				onBehalfOf
			)
			if (alreadyExists) {
				await updateFn({ path, workspace: workspaceTo, requestBody: data } as any)
			} else {
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
	// Triggers and schedules are frontend-specific
	if (kind === 'schedule') {
		return ScheduleService.existsSchedule({ workspace, path })
	} else if (kind === 'trigger') {
		const triggersKind: TriggerKind[] = [
			'kafka',
			'mqtt',
			'nats',
			'postgres',
			'routes',
			'schedules',
			'sqs',
			'websockets',
			'gcp'
		]
		if (
			additionalInformation?.triggers &&
			triggersKind.includes(additionalInformation.triggers.kind)
		) {
			return existsTrigger({ workspace, path }, additionalInformation.triggers.kind)
		} else {
			throw new Error(
				`Unexpected triggers kind, expected one of: '${triggersKind.join(', ')}' got: ${additionalInformation?.triggers?.kind}`
			)
		}
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
	// Triggers are frontend-specific
	if (kind === 'trigger') {
		if (additionalInformation?.triggers) {
			try {
				return await getTriggerValue(additionalInformation.triggers.kind, path, workspace)
			} catch {
				return {}
			}
		}
		return {}
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
	// Triggers are frontend-specific
	if (kind === 'trigger' && additionalInformation?.triggers) {
		try {
			return await getTriggerPermissionedAs(additionalInformation.triggers.kind, path, workspace)
		} catch {
			return undefined
		}
	}

	return sharedGetOnBehalfOf(makeProvider(), kind as DeployKind, path, workspace)
}
