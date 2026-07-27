import { AmqpTriggerService, type EditAmqpTrigger } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { get, type Writable } from 'svelte/store'

export async function saveAmqpTriggerFromCfg(
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
	const requestBody: EditAmqpTrigger = {
		amqp_resource_path: cfg.amqp_resource_path,
		queue_name: cfg.queue_name,
		exchange: cfg.exchange,
		options: cfg.options,
		path: cfg.path,
		script_path: cfg.script_path,
		is_flow: cfg.is_flow,
		mode: cfg.mode,
		permissioned_as: cfg.permissioned_as,
		preserve_permissioned_as: cfg.preserve_permissioned_as,
		...errorHandlerAndRetries
	}
	try {
		if (edit) {
			await AmqpTriggerService.updateAmqpTrigger({
				workspace,
				path: initialPath,
				requestBody
			})
			sendUserToast(`AMQP trigger ${cfg.path} updated`)
		} else {
			await AmqpTriggerService.createAmqpTrigger({
				workspace,
				requestBody: { ...requestBody, mode: 'enabled' }
			})
			sendUserToast(`AMQP trigger ${cfg.path} created`)
		}

		if (!get(usedTriggerKinds).includes('amqp')) {
			usedTriggerKinds.update((t) => [...t, 'amqp'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
