import { WebsocketTriggerService, type EditWebsocketTrigger } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type { Writable } from 'svelte/store'
import { get } from 'svelte/store'

export async function saveWebsocketTriggerFromCfg(
	initialPath: string,
	triggerCfg: Record<string, any>,
	edit: boolean,
	workspace: string,
	usedTriggerKinds: Writable<string[]>
): Promise<boolean> {
	const errorHandlerAndRetries = !triggerCfg.is_flow
		? {
				error_handler_path: triggerCfg.error_handler_path,
				error_handler_args: triggerCfg.error_handler_path
					? triggerCfg.error_handler_args
					: undefined,
				retry: triggerCfg.retry,
			}
		: {}
	const requestBody: EditWebsocketTrigger = {
		path: triggerCfg.path,
		script_path: triggerCfg.script_path,
		is_flow: triggerCfg.is_flow,
		url: triggerCfg.url,
		filters: triggerCfg.filters,
		initial_messages: triggerCfg.initial_messages,
		url_runnable_args: triggerCfg.url_runnable_args,
		can_return_message: triggerCfg.can_return_message,
		...errorHandlerAndRetries
	}
	try {
		if (edit) {
			await WebsocketTriggerService.updateWebsocketTrigger({
				workspace: workspace,
				path: initialPath,
				requestBody: requestBody
			})
			sendUserToast(`Websocket trigger ${triggerCfg.path} updated`)
		} else {
			await WebsocketTriggerService.createWebsocketTrigger({
				workspace: workspace,
				requestBody: { ...requestBody, enabled: true }
			})
			sendUserToast(`Websocket trigger ${triggerCfg.path} created`)
		}
		if (!get(usedTriggerKinds).includes('ws')) {
			usedTriggerKinds.update((t) => [...t, 'ws'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
