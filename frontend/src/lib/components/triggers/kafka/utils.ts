import { KafkaTriggerService, type EditKafkaTrigger } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { get, type Writable } from 'svelte/store'

export async function saveKafkaTriggerFromCfg(
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
				retry: cfg.retry,
			}
		: {}
	const requestBody: EditKafkaTrigger = {
		path: cfg.path,
		script_path: cfg.script_path,
		is_flow: cfg.is_flow,
		kafka_resource_path: cfg.kafka_resource_path,
		group_id: cfg.group_id,
		topics: cfg.topics,
		...errorHandlerAndRetries
	}
	try {
		if (edit) {
			await KafkaTriggerService.updateKafkaTrigger({
				workspace,
				path: initialPath,
				requestBody
			})
			sendUserToast(`Kafka trigger ${cfg.path} updated`)
		} else {
			await KafkaTriggerService.createKafkaTrigger({
				workspace,
				requestBody: { ...requestBody, enabled: true }
			})
			sendUserToast(`Kafka trigger ${cfg.path} created`)
		}
		if (!get(usedTriggerKinds).includes('kafka')) {
			usedTriggerKinds.update((t) => [...t, 'kafka'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
