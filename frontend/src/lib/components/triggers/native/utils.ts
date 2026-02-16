import type { NativeServiceName, NativeTrigger, NativeTriggerData } from '$lib/gen/types.gen'
import { isCloudHosted } from '$lib/cloud'
import { NativeTriggerService, WorkspaceIntegrationService } from '$lib/gen'
import { get, type Writable } from 'svelte/store'
import { sendUserToast } from '$lib/toast'

export interface NativeTriggerConfig {
	readonly serviceDisplayName: string
	readonly serviceKey: NativeServiceName
	readonly supportsSync: boolean
	readonly isCloudCompatible: boolean
	readonly templates?: {
		script?: string
		flow?: string
	}
}

export const NATIVE_TRIGGER_SERVICES: Record<NativeServiceName, NativeTriggerConfig> = {
	nextcloud: {
		serviceDisplayName: 'Nextcloud',
		serviceKey: 'nextcloud',
		supportsSync: true,
		isCloudCompatible: true,
		templates: {
			script: '/scripts/add?hub=hub%2F28115',
			flow: '/flows/add?hub=73'
		}
	},
	// Google handles both Drive and Calendar triggers via trigger_type in service_config
	google: {
		serviceDisplayName: 'Google',
		serviceKey: 'google',
		supportsSync: true,
		isCloudCompatible: true,
		templates: {
			script: '/scripts/add?hub=hub%2F28135',
			flow: '/flows/add?hub=75'
		}
	}
}

export async function isServiceAvailable(
	service: NativeServiceName,
	workspace: string
): Promise<boolean> {
	const config = NATIVE_TRIGGER_SERVICES[service]
	if (!config) return false

	if (isCloudHosted() && !config.isCloudCompatible) {
		return false
	}

	try {
		const response = await WorkspaceIntegrationService.checkIfNativeTriggersServiceExists({
			workspace,
			serviceName: service
		})

		return response
	} catch (workspaceErr) {
		console.debug(`Workspace integration check failed for ${service}:`, workspaceErr)
		return false
	}
}

export function getAvailableServices(): NativeServiceName[] {
	return Object.keys(NATIVE_TRIGGER_SERVICES) as NativeServiceName[]
}

export async function getAvailableNativeTriggerServices(
	workspace: string
): Promise<NativeServiceName[]> {
	const services = getAvailableServices()
	const availableServices: NativeServiceName[] = []

	for (const service of services) {
		const available = await isServiceAvailable(service, workspace)
		if (available) {
			availableServices.push(service)
		}
	}

	return availableServices
}

export function getServiceConfig(service: NativeServiceName): NativeTriggerConfig | undefined {
	return NATIVE_TRIGGER_SERVICES[service]
}

// NativeTrigger now has script_path and is_flow directly
// This type adds the marked property for search highlighting
export type ExtendedNativeTrigger = NativeTrigger & { marked?: string }

export interface ServiceFormProps {
	config: Record<string, any>
	errors: Record<string, string>
	resources: Array<{ path: string; description?: string }>
	onConfigChange: (newConfig: Record<string, any>) => void
	onTest?: () => Promise<void>
	disabled?: boolean
}

export function validateCommonFields(config: Record<string, any>): Record<string, string> {
	const errors: Record<string, string> = {}

	if (!config.script_path?.trim()) {
		errors.script_path = 'Script/Flow path is required'
	}

	return errors
}

export function formatTriggerDisplayName(trigger: NativeTrigger): string {
	return `${trigger.script_path} (external id: ${trigger.external_id})`
}

export function getTriggerIconName(service: NativeServiceName): string {
	switch (service) {
		case 'nextcloud':
			return 'NextcloudIcon'
		case 'google':
			return 'GoogleIcon'
		default:
			return 'NextcloudIcon'
	}
}

export async function getServiceIcon(service: NativeServiceName): Promise<any> {
	switch (service) {
		case 'nextcloud':
			return (await import('$lib/components/icons/NextcloudIcon.svelte')).default
		case 'google':
			return (await import('$lib/components/icons/GoogleIcon.svelte')).default
	}
}

export function getServiceTemplates(
	service: NativeServiceName
): { script?: string; flow?: string } | undefined {
	const config = getServiceConfig(service)
	return config?.templates
}

export function getTemplatePath(
	service: NativeServiceName,
	type: 'script' | 'flow'
): string | undefined {
	const templates = getServiceTemplates(service)
	return templates?.[type]
}

export interface NextcloudEvent {
	path: string
	description?: string
}

export function getNextcloudSchema(availableEvents: NextcloudEvent[]) {
	return {
		type: 'object',
		properties: {
			event: {
				type: 'string',
				title: 'Event',
				description: 'The type of Nextcloud event to listen for',
				enum: availableEvents.map((e) => e.path),
				enumLabels: availableEvents.reduce(
					(acc, cur) => ({ ...acc, [cur.path]: cur.description ?? cur.path }),
					{} as Record<string, string>
				)
			},
			eventFilter: {
				type: 'object',
				title: 'Event filter',
				description: 'Optional filter criteria for the event (JSON object)'
			},
			userIdFilter: {
				type: 'string',
				title: 'User ID filter',
				description: 'Filter events by specific user ID'
			},
			headers: {
				type: 'object',
				title: 'Headers',
				description: 'Optional HTTP headers to include (JSON object)'
			}
		},
		required: ['event']
	}
}

export async function saveNativeTriggerFromCfg(
	service: NativeServiceName,
	initialExternalId: string,
	triggerCfg: Record<string, any>,
	edit: boolean,
	workspace: string,
	usedTriggerKinds: Writable<string[]>
): Promise<string | null> {
	const requestBody: NativeTriggerData = {
		script_path: triggerCfg.script_path,
		is_flow: triggerCfg.is_flow,
		service_config: triggerCfg.service_config
	}

	const serviceName = NATIVE_TRIGGER_SERVICES[service].serviceDisplayName

	try {
		let externalId = initialExternalId
		if (edit) {
			await NativeTriggerService.updateNativeTrigger({
				workspace: workspace,
				serviceName: service,
				externalId: initialExternalId,
				requestBody
			})
			sendUserToast(`${serviceName} trigger ${externalId} updated`)
		} else {
			const response = await NativeTriggerService.createNativeTrigger({
				workspace: workspace,
				serviceName: service,
				requestBody
			})
			externalId = response.external_id
			sendUserToast(`${serviceName} trigger ${externalId} created`)
		}
		if (!get(usedTriggerKinds).includes(service)) {
			usedTriggerKinds.update((t) => [...t, service])
		}
		return externalId
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return null
	}
}
