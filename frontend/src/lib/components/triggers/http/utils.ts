import { base } from '$lib/base'
import { isCloudHosted } from '$lib/cloud'

export const SECRET_KEY_PATH = 'secret_key_path'
export const HUB_SCRIPT_ID = 19661
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
