import { base } from '$lib/base'
import { isCloudHosted } from '$lib/cloud'
import { HttpTriggerService } from '$lib/gen/services.gen'
import { sendUserToast } from '$lib/toast'
import type { Writable } from 'svelte/store'
import { get } from 'svelte/store'

export const SECRET_KEY_PATH = 'secret_key_path'
export const HUB_SCRIPT_ID = 19670
export const SIGNATURE_TEMPLATE_SCRIPT_HUB_PATH: string = `hub/${HUB_SCRIPT_ID}`
export const SIGNATURE_TEMPLATE_FLOW_HUB_ID = '67'

export function getHttpRoute(
	route_prefix: string,
	route_path: string | undefined,
	workspaced_route: boolean,
	workspace_id: string
) {
	return `${location.origin}${base}/api/${route_prefix}/${
		isCloudHosted() || workspaced_route ? workspace_id + '/' : ''
	}${route_path ?? ''}`
}

export function replacePlaceholderForSignatureScriptTemplate(content: string) {
	const params = new URLSearchParams(window.location.search)
	const secret_key_path = params.get(SECRET_KEY_PATH) ?? ''
	return content.replace(
		/(const\s+SECRET_KEY_VARIABLE_PATH\s*=\s*")[^"]*(";)/,
		`$1${secret_key_path}$2`
	)
}

export async function saveHttpRouteFromCfg(
	initialPath: string,
	routeCfg: Record<string, any>,
	edit: boolean,
	workspace: string,
	isAdmin: boolean,
	usedTriggerKinds: Writable<string[]>
): Promise<boolean> {
	const requestBody = {
		path: routeCfg.path,
		script_path: routeCfg.script_path,
		is_flow: routeCfg.is_flow,
		is_async: routeCfg.is_async,
		authentication_method: routeCfg.authentication_method,
		route_path: isAdmin || !edit ? routeCfg.route_path : undefined,
		http_method: routeCfg.http_method,
		is_static_website: routeCfg.is_static_website,
		workspaced_route: routeCfg.workspaced_route,
		authentication_resource_path: routeCfg.authentication_resource_path,
		wrap_body: routeCfg.wrap_body,
		raw_string: routeCfg.raw_string,
		description: routeCfg.description,
		summary: routeCfg.summary
	}
	try {
		if (edit) {
			await HttpTriggerService.updateHttpTrigger({
				workspace: workspace,
				path: initialPath,
				requestBody: requestBody
			})
			sendUserToast(`Route ${routeCfg.path} updated`)
		} else {
			await HttpTriggerService.createHttpTrigger({
				workspace: workspace,
				requestBody: requestBody
			})
			sendUserToast(`Route ${routeCfg.path} created`)
		}
		if (!get(usedTriggerKinds).includes('http')) {
			usedTriggerKinds.update((t) => [...t, 'http'])
		}
		return true
	} catch (error) {
		sendUserToast(error.body || error.message, true)
		return false
	}
}
