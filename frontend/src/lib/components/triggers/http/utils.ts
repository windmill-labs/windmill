import { base } from '$lib/base'
import { isCloudHosted } from '$lib/cloud'
import { random_adj } from '$lib/components/random_positive_adjetive'
import type { HttpMethod, NewHttpTrigger } from '$lib/gen'
import { HttpTriggerService } from '$lib/gen/services.gen'
import { sendUserToast } from '$lib/toast'
import { generateRandomString, OpenApi as WindmillOpenApi } from '$lib/utils'
import { type OpenAPI } from 'openapi-types'
import type { Writable } from 'svelte/store'
import { get } from 'svelte/store'

export const HTTP_ROUTE_DEFAULT_ALLOWED_ORIGINS_SETTING = 'http_route_default_allowed_origins'

/** Split the comma-separated form the origins field and the setting both use. */
export function parseAllowedOrigins(raw: string): string[] {
	return raw
		.split(',')
		.map((origin) => origin.trim())
		.filter((origin) => origin !== '')
}

/**
 * Read the instance-default setting, mirroring `parse_allowed_origins_setting`
 * in windmill-common: the settings UI writes a comma-separated string, but the
 * API accepts an array too.
 */
export function parseAllowedOriginsSetting(setting: unknown): string[] {
	if (typeof setting === 'string') return parseAllowedOrigins(setting)
	if (Array.isArray(setting))
		return setting
			.filter((origin): origin is string => typeof origin === 'string')
			.map((origin) => origin.trim())
			.filter((origin) => origin !== '')
	return []
}

/**
 * Whether a route is restricted to specific origins, mirroring
 * `effective_allowed_origins` in windmill-trigger-http: a route with its own
 * list restricts, an empty one included since it then matches no origin at all;
 * only a route without one falls back to the instance default, and `*` in
 * either is the opt-out.
 */
export function isOriginRestricted(
	allowed_origins: string[] | undefined,
	instanceDefaultOrigins: string[]
): boolean {
	if (allowed_origins !== undefined) return !allowed_origins.includes('*')
	return instanceDefaultOrigins.length > 0 && !instanceDefaultOrigins.includes('*')
}

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
	_isAdmin: boolean,
	usedTriggerKinds: Writable<string[]>
): Promise<boolean> {
	const requestBody: NewHttpTrigger = {
		path: routeCfg.path,
		script_path: routeCfg.script_path,
		is_flow: routeCfg.is_flow,
		request_type: routeCfg.request_type,
		authentication_method: routeCfg.authentication_method,
		route_path: routeCfg.route_path,
		http_method: routeCfg.http_method,
		is_static_website: routeCfg.is_static_website,
		static_asset_config: routeCfg.static_asset_config,
		workspaced_route: routeCfg.workspaced_route,
		authentication_resource_path: routeCfg.authentication_resource_path,
		wrap_body: routeCfg.wrap_body,
		raw_string: routeCfg.raw_string,
		allowed_origins: routeCfg.allowed_origins,
		description: routeCfg.description,
		summary: routeCfg.summary,
		error_handler_path: routeCfg.error_handler_path,
		error_handler_args: routeCfg.error_handler_path ? routeCfg.error_handler_args : undefined,
		retry: routeCfg.retry,
		mode: routeCfg.mode,
		permissioned_as: routeCfg.permissioned_as,
		preserve_permissioned_as: routeCfg.preserve_permissioned_as
	}
	try {
		if (edit) {
			await HttpTriggerService.updateHttpTrigger({
				workspace: workspace,
				path: initialPath,
				requestBody: {
					...requestBody,
					route_path: routeCfg.route_path
				}
			})
			sendUserToast(`Route ${routeCfg.path} updated`)
		} else {
			await HttpTriggerService.createHttpTrigger({
				workspace: workspace,
				requestBody: { ...requestBody, mode: 'enabled' }
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

export type Source = 'OpenAPI' | 'OpenAPI_File' | 'OpenAPI_URL'

function convertOpenApiPathToRoutePath(openApiPath: string) {
	return openApiPath.replace(/{([^}]+)}/g, ':$1').slice(1)
}

const MAX_PATH_LEN = 255

function generateFolderPath(folderName: string, summary?: string) {
	let suffix: string
	const prefix = `f/${folderName}/`
	if (!summary) {
		suffix = `${random_adj()}_${generateRandomString(6)}`
	} else {
		const remainingLen = MAX_PATH_LEN - prefix.length
		if (summary.length > remainingLen) {
			suffix = summary.substring(0, remainingLen).replaceAll(' ', '_')
		} else {
			suffix = summary.replaceAll(' ', '_')
		}
	}

	return prefix.concat(suffix).toLocaleLowerCase()
}

function processOpenApiDocument(
	document: OpenAPI.Document,
	folderName: string,
	_version?: WindmillOpenApi.OpenApiVersion
) {
	const paths = document.paths

	const httpTrigger: NewHttpTrigger[] = []

	for (const path in paths) {
		const pathItem = paths[path]
		if (!pathItem) continue

		const methods: HttpMethod[] = ['get', 'post', 'put', 'patch', 'delete']

		for (const method of methods) {
			const routeDetail = pathItem[method]
			if (!routeDetail) continue

			httpTrigger.push({
				route_path: convertOpenApiPathToRoutePath(path),
				http_method: method,
				authentication_method: 'none',
				workspaced_route: false,
				request_type: 'async',
				script_path: '',
				raw_string: false,
				is_flow: false,
				is_static_website: false,
				wrap_body: false,
				path: generateFolderPath(folderName, routeDetail.summary)
			})
		}
	}

	return httpTrigger
}

export async function generateHttpTriggerFromOpenApi(
	api: string,
	folderName: string
): Promise<NewHttpTrigger[]> {
	const [document] = await WindmillOpenApi.parse(api)

	return processOpenApiDocument(document, folderName)
}
