import { SqsTriggerService, type EditSqsTrigger } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { get, type Writable } from 'svelte/store'

export async function saveSqsTriggerFromCfg(
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
	const requestBody: EditSqsTrigger = {
		path: cfg.path,
		script_path: cfg.script_path,
		is_flow: cfg.is_flow,
		aws_resource_path: cfg.aws_resource_path,
		queue_url: cfg.queue_url,
		message_attributes: cfg.message_attributes,
		aws_auth_resource_type: cfg.aws_auth_resource_type,
		enabled: cfg.enabled,
		...errorHandlerAndRetries
	}
	try {
		if (edit) {
			await SqsTriggerService.updateSqsTrigger({
				workspace,
				path: initialPath,
				requestBody
			})
			sendUserToast(`SQS trigger ${cfg.path} updated`)
		} else {
			await SqsTriggerService.createSqsTrigger({
				workspace,
				requestBody: { ...requestBody, enabled: true }
			})
			sendUserToast(`SQS trigger ${cfg.path} created`)
		}

		if (!get(usedTriggerKinds).includes('sqs')) {
			usedTriggerKinds.update((t) => [...t, 'sqs'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
