import { isCloudHosted } from '$lib/cloud'
import { type NewEmailTrigger, EmailTriggerService, SettingService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type { Writable } from 'svelte/store'
import { get } from 'svelte/store'

export function getEmailAddress(
	local_part: string | undefined,
	workspaced_local_part: boolean,
	workspace_id: string,
	emailDomain: string
) {
	return `${isCloudHosted() || workspaced_local_part ? workspace_id + '-' : ''}${local_part ?? ''}@${emailDomain}`
}

export async function saveEmailTriggerFromCfg(
	initialPath: string,
	routeCfg: Record<string, any>,
	edit: boolean,
	workspace: string,
	isAdmin: boolean,
	usedTriggerKinds: Writable<string[]>
): Promise<boolean> {
	const requestBody: NewEmailTrigger = {
		path: routeCfg.path,
		script_path: routeCfg.script_path,
		local_part: routeCfg.local_part,
		is_flow: routeCfg.is_flow,
		workspaced_local_part: routeCfg.workspaced_local_part,
		error_handler_path: routeCfg.error_handler_path,
		error_handler_args: routeCfg.error_handler_path ? routeCfg.error_handler_args : undefined,
		retry: routeCfg.retry
	}
	try {
		if (edit) {
			await EmailTriggerService.updateEmailTrigger({
				workspace: workspace,
				path: initialPath,
				requestBody: {
					...requestBody,
					local_part: isAdmin || !edit ? routeCfg.local_part : undefined
				}
			})
			sendUserToast(`Route ${routeCfg.path} updated`)
		} else {
			await EmailTriggerService.createEmailTrigger({
				workspace: workspace,
				requestBody: requestBody
			})
			sendUserToast(`Route ${routeCfg.path} created`)
		}
		if (!get(usedTriggerKinds).includes('email')) {
			usedTriggerKinds.update((t) => [...t, 'email'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}

export async function getEmailDomain(): Promise<string> {
	return (
		((await SettingService.getGlobal({
			key: 'email_domain'
		})) as any) ?? 'mail.test.com'
	)
}
