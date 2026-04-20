import { AzureTriggerService, type AzureTriggerData } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { get, type Writable } from 'svelte/store'

export async function saveAzureTriggerFromCfg(
	initialPath: string,
	cfg: Record<string, any>,
	edit: boolean,
	workspace: string,
	usedTriggerKinds: Writable<string[]>
): Promise<boolean> {
	try {
		const errorHandlerAndRetries = !cfg.is_flow
			? {
					error_handler_path: cfg.error_handler_path,
					error_handler_args: cfg.error_handler_path ? cfg.error_handler_args : undefined,
					retry: cfg.retry
				}
			: {}

		const requestBody: AzureTriggerData = {
			azure_resource_path: cfg.azure_resource_path,
			azure_mode: cfg.azure_mode,
			scope_resource_id: cfg.scope_resource_id,
			topic_name: cfg.topic_name,
			subscription_name: cfg.subscription_name,
			subscription_mode: cfg.subscription_mode,
			base_endpoint: cfg.base_endpoint,
			event_type_filters: cfg.event_type_filters,
			advanced_filter: cfg.advanced_filter,
			delivery_config: cfg.delivery_config,
			max_events: cfg.max_events,
			max_wait_time_sec: cfg.max_wait_time_sec,
			auto_acknowledge_msg: cfg.auto_acknowledge_msg,
			path: cfg.path,
			script_path: cfg.script_path,
			mode: cfg.mode,
			is_flow: cfg.is_flow,
			permissioned_as: cfg.permissioned_as,
			preserve_permissioned_as: cfg.preserve_permissioned_as,
			...errorHandlerAndRetries
		}
		if (edit) {
			await AzureTriggerService.updateAzureTrigger({
				workspace,
				path: initialPath,
				requestBody
			})
			sendUserToast(`Azure Event Grid trigger ${cfg.path} updated`)
		} else {
			await AzureTriggerService.createAzureTrigger({
				workspace,
				requestBody: {
					...requestBody,
					mode: 'enabled'
				}
			})
			sendUserToast(`Azure Event Grid trigger ${cfg.path} created`)
		}

		if (!get(usedTriggerKinds).includes('azure')) {
			usedTriggerKinds.update((t) => [...t, 'azure'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
