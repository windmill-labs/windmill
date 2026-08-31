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
 * Entries the API refuses, mirroring `validate_allowed_origins`.
 *
 * Deliberately short: a stored origin is only ever compared against the
 * request's `Origin`, so a shape that cannot match is dead config rather than a
 * risk. `null` is the exception, since it is what every sandboxed iframe sends.
 */
export function allowedOriginRejection(origin: string): string | undefined {
	// Same order as `validate_allowed_origins`, so the same entry draws the same
	// message on both sides rather than only the same verdict.
	if (origin === '*') return undefined
	if (origin === '') return 'An origin must not be empty'
	if (origin.length > MAX_ALLOWED_ORIGIN_LEN)
		return `'${origin.slice(0, 40)}…' is longer than any origin a browser sends`
	if (origin.includes(','))
		return `'${origin}' must not contain a comma, which separates entries`
	if (origin.toLowerCase() === 'null')
		return `'null' is what a sandboxed iframe sends, so it would allow any page that can open one`
	if (!/^[\x21-\x7e]+$/.test(origin))
		return `'${origin}' must contain only visible ASCII, with no whitespace`
	return undefined
}

/** Kept in step with `MAX_ALLOWED_ORIGIN{,S}` in windmill-common. */
export const MAX_ALLOWED_ORIGINS = 100
export const MAX_ALLOWED_ORIGIN_LEN = 256

/**
 * The first entry the API would refuse, if any. Derived from the stored list
 * rather than the field, so it stays correct while the editor is on another tab
 * and the field is not mounted. An empty list is not an error: it resolves as an
 * unset one, so there is nothing in it to refuse.
 *
 * A comma-bearing entry does reach this, through the settings path where a list
 * can be given as an array. It cannot arrive from the origins field, which
 * splits on commas before this ever sees it.
 */
export function allowedOriginsError(allowed_origins: string[] | undefined): string | undefined {
	if (allowed_origins !== undefined && allowed_origins.length > MAX_ALLOWED_ORIGINS)
		return `At most ${MAX_ALLOWED_ORIGINS} origins, got ${allowed_origins.length}`
	return allowed_origins?.map(allowedOriginRejection).find((message) => message !== undefined)
}

/**
 * Shapes that save fine but can never equal an `Origin` header, so the route
 * would read as configured while allowing nothing.
 *
 * Advisory only. What a browser sends is the caller's to know, so this points
 * at the usual slips rather than deciding which origins are legitimate.
 */
export function allowedOriginWarning(origin: string): string | undefined {
	if (origin === '*' || allowedOriginRejection(origin) !== undefined) return undefined
	const separator = origin.indexOf('://')
	if (separator <= 0) return `'${origin}' has no scheme, such as https://`
	const rest = origin.slice(separator + 3)
	if (rest === '') return `'${origin}' has no host`
	if (/[/?#]/.test(rest))
		return `'${origin}' should be scheme://host[:port], with no path, query or fragment`
	if (rest.includes('@')) return `'${origin}' should not contain userinfo`
	// Only the port is checked past this point. The host is left alone on
	// purpose: browsers send origins this cannot anticipate, `chrome-extension`
	// and IPv6 literals among them, and a warning that cries wolf on a working
	// origin is worse than one that stays quiet.
	if (rest.startsWith(':')) return `'${origin}' has no host`
	// An unclosed bracket would otherwise leave `portStart` at zero, which reads
	// as "no port" and lets the entry through unremarked.
	if (rest.startsWith('[') && !rest.includes(']'))
		return `'${origin}' has an unclosed IPv6 host`
	const portStart = rest.startsWith('[') ? rest.indexOf(']') + 1 : rest.indexOf(':')
	// A trailing colon is a port, an empty one — distinct from having none.
	const port = portStart > 0 && rest[portStart] === ':' ? rest.slice(portStart + 1) : undefined
	if (port !== undefined && !(/^[0-9]{1,5}$/.test(port) && Number(port) <= 65535))
		return `'${origin}' has a port no browser can send`
	return undefined
}

/**
 * What the settings API would refuse, mirroring `parse_allowed_origins_setting`
 * in windmill-common.
 *
 * Distinct from reading the setting for display: that drops entries it cannot
 * use, while this has to report them, or a shape only the YAML editor can
 * produce would pass here and come back as a 400 on save.
 */
export function allowedOriginsSettingError(setting: unknown): string | undefined {
	let origins: string[]
	if (setting == null || typeof setting === 'string') {
		origins = parseAllowedOrigins(typeof setting === 'string' ? setting : '')
	} else if (Array.isArray(setting)) {
		if (setting.some((entry) => typeof entry !== 'string')) return 'Entries must be strings'
		// Not filtered for empties, unlike the comma-separated form, where a
		// trailing separator is a typing artifact rather than an entry.
		origins = setting.map((entry) => (entry as string).trim())
	} else {
		return 'Expected a comma-separated string or a list of strings'
	}
	return allowedOriginsError(origins)
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
 * `effective_allowed_origins` in windmill-trigger-http: a route with a non-empty
 * list of its own restricts, `*` in it is the opt-out, and anything else — an
 * empty list included, since that is not a configuration — falls back to the
 * instance default.
 */
export function isOriginRestricted(
	allowed_origins: string[] | undefined,
	instanceDefaultOrigins: string[]
): boolean {
	if (allowed_origins !== undefined && allowed_origins.length > 0)
		return !allowed_origins.includes('*')
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
