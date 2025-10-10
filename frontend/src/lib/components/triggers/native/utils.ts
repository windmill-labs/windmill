import type { NativeServiceName, NativeTrigger } from '$lib/gen/types.gen'
import { isCloudHosted } from '$lib/cloud'
import { SettingService } from '$lib/gen/services.gen'

export interface NativeTriggerConfig {
	readonly serviceDisplayName: string
	readonly serviceKey: NativeServiceName
	readonly resourceType: string
	readonly supportsSync: boolean
	readonly supportsFetchConfig: boolean
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
		resourceType: 'nextcloud',
		supportsSync: true,
		supportsFetchConfig: true,
		isCloudCompatible: true,
		templates: {
			script: 'hub/19824/nextcloud-script-with-preprocessor-template-windmill',
			flow: 'hub/19824/nextcloud-flow-with-preprocessor-template-windmill'
		}
	}
}

export async function isServiceAvailable(service: NativeServiceName): Promise<boolean> {
	const config = NATIVE_TRIGGER_SERVICES[service]
	if (!config) return false

	if (isCloudHosted() && !config.isCloudCompatible) {
		return false
	}

	try {
		const availableClients = (await SettingService.getGlobal({ key: 'oauths' })) as Record<
			string,
			unknown
		>
		return availableClients[service] != undefined
	} catch (err) {
		console.debug(`Service ${service} not available:`, err)
		return false
	}
}

export function getAvailableServices(): NativeServiceName[] {
	return Object.keys(NATIVE_TRIGGER_SERVICES) as NativeServiceName[]
}

export async function getAvailableNativeTriggerServices(): Promise<NativeServiceName[]> {
	const services = getAvailableServices()
	const availableServices: NativeServiceName[] = []

	for (const service of services) {
		const available = await isServiceAvailable(service)
		if (available) {
			availableServices.push(service)
		}
	}

	return availableServices
}

export function getServiceConfig(service: NativeServiceName): NativeTriggerConfig | undefined {
	return NATIVE_TRIGGER_SERVICES[service]
}

export interface ExtendedNativeTrigger extends NativeTrigger {
	id: number
	runnable_path: string
	runnable_kind: 'script' | 'flow'
}


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

	if (!config.runnable_path?.trim()) {
		errors.runnable_path = 'Script/Flow path is required'
	}

	if (!config.resource_path?.trim()) {
		errors.resource_path = 'Resource path is required'
	}

	return errors
}

export function formatTriggerDisplayName(trigger: NativeTrigger): string {
	const serviceConfig = getServiceConfig(trigger.service_name)
	const serviceName = serviceConfig?.serviceDisplayName || trigger.service_name
	return `${serviceName} - ${trigger.summary || trigger.external_id}`
}

export function getTriggerIconName(service: NativeServiceName): string {
	switch (service) {
		case 'nextcloud':
			return 'NextcloudIcon'
		default:
			return 'Database'
	}
}

export async function getServiceIcon(service: NativeServiceName): Promise<any> {
	switch (service) {
		case 'nextcloud':
			return (await import('$lib/components/icons/NextcloudIcon.svelte')).default
	}
}

export function getServiceTemplates(service: NativeServiceName): { script?: string; flow?: string } | undefined {
	const config = getServiceConfig(service)
	return config?.templates
}

export function getTemplateUrl(service: NativeServiceName, type: 'script' | 'flow'): string | undefined {
	const templates = getServiceTemplates(service)
	return templates?.[type]
}
