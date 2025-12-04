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
	emailCfg: Record<string, any>,
	edit: boolean,
	workspace: string,
	isAdmin: boolean,
	usedTriggerKinds: Writable<string[]>
): Promise<boolean> {
	const requestBody: NewEmailTrigger = {
		path: emailCfg.path,
		script_path: emailCfg.script_path,
		local_part: emailCfg.local_part,
		is_flow: emailCfg.is_flow,
		workspaced_local_part: emailCfg.workspaced_local_part,
		error_handler_path: emailCfg.error_handler_path,
		error_handler_args: emailCfg.error_handler_path ? emailCfg.error_handler_args : undefined,
		mode: emailCfg.mode,
		retry: emailCfg.retry
	}
	try {
		if (edit) {
			await EmailTriggerService.updateEmailTrigger({
				workspace: workspace,
				path: initialPath,
				requestBody: {
					...requestBody,
					local_part: isAdmin || !edit ? emailCfg.local_part : undefined
				}
			})
			sendUserToast(`Email trigger ${emailCfg.path} updated`)
		} else {
			await EmailTriggerService.createEmailTrigger({
				workspace: workspace,
				requestBody: { ...requestBody, mode: 'enabled' }
			})
			sendUserToast(`Email trigger ${emailCfg.path} created`)
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
