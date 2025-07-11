import { MqttTriggerService, type EditMqttTrigger } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { get, type Writable } from 'svelte/store'

export async function saveMqttTriggerFromCfg(
	initialPath: string,
	cfg: Record<string, any>,
	edit: boolean,
	workspace: string,
	usedTriggerKinds: Writable<string[]>
): Promise<boolean> {
	const errorHandlerAndRetries = !cfg.is_flow
		? {
				error_handler_path: cfg.error_handler_path,
				error_handler_args: cfg.error_handler_path ? cfg.error_handler_args : undefined,
				retry: cfg.retry
			}
		: {}
	const requestBody: EditMqttTrigger = {
		client_id: cfg.client_id,
		client_version: cfg.client_version,
		v3_config: cfg.v3_config,
		v5_config: cfg.v5_config,
		mqtt_resource_path: cfg.mqtt_resource_path,
		subscribe_topics: cfg.subscribe_topics,
		path: cfg.path,
		script_path: cfg.script_path,
		enabled: cfg.enabled,
		is_flow: cfg.is_flow,
		...errorHandlerAndRetries
	}
	try {
		if (edit) {
			await MqttTriggerService.updateMqttTrigger({
				workspace,
				path: initialPath,
				requestBody
			})
			sendUserToast(`MQTT trigger ${cfg.path} updated`)
		} else {
			await MqttTriggerService.createMqttTrigger({
				workspace,
				requestBody: { ...requestBody, enabled: true }
			})
			sendUserToast(`MQTT trigger ${cfg.path} created`)
		}

		if (!get(usedTriggerKinds).includes('mqtt')) {
			usedTriggerKinds.update((t) => [...t, 'mqtt'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
