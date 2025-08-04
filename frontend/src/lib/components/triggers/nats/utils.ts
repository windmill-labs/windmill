import { NatsTriggerService, type EditNatsTrigger } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { get, type Writable } from 'svelte/store'

export async function saveNatsTriggerFromCfg(
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
				email_recipients: cfg.email_recipients
			}
		: {}
	const requestBody: EditNatsTrigger = {
		path: cfg.path,
		script_path: cfg.script_path,
		is_flow: cfg.is_flow,
		nats_resource_path: cfg.nats_resource_path,
		stream_name: cfg.stream_name,
		consumer_name: cfg.consumer_name,
		subjects: cfg.subjects,
		use_jetstream: cfg.use_jetstream,
		...errorHandlerAndRetries
	}
	try {
		if (edit) {
			await NatsTriggerService.updateNatsTrigger({
				workspace,
				path: initialPath,
				requestBody
			})
			sendUserToast(`Nats trigger ${cfg.path} updated`)
		} else {
			await NatsTriggerService.createNatsTrigger({
				workspace,
				requestBody: {
					...requestBody,
					enabled: true
				}
			})
			sendUserToast(`Nats trigger ${cfg.path} created`)
		}
		if (!get(usedTriggerKinds).includes('nats')) {
			usedTriggerKinds.update((t) => [...t, 'nats'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
