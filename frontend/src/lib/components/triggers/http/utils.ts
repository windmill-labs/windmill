import { base } from '$lib/base'
import { isCloudHosted } from '$lib/cloud'
import { random_adj } from '$lib/components/random_positive_adjetive'
import type { HttpMethod, HttpTrigger, NewHttpTrigger } from '$lib/gen'
import { HttpTriggerService } from '$lib/gen/services.gen'
import { sendUserToast } from '$lib/toast'
import { generateRandomString, OpenApi as WindmillOpenApi } from '$lib/utils'
import { type OpenAPI, type OpenAPIV3_1 } from 'openapi-types'
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
				is_async: true,
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

export const INFO_TEMPLATE = `{
  "title": "My API",
  "version": "1.0.0",
  "description": "A short description of your API",
  "contact": {
    "name": "Jane Doe",
    "email": "jane@example.com",
    "url": "https://example.com"
  },
  "license": {
    "name": "MIT",
    "url": "https://opensource.org/licenses/MIT"
  }
}`

export const SERVER_TEMPLATES = `[
  {
    "url": "https://api.example.com",
    "description": "Production server"
  },
  {
    "url": "https://staging-api.example.com",
    "description": "Staging server"
  }
]`

type OpenAPIParameter = {
	name: string
	in: 'path'
	required: true
	schema: {
		type: 'string' // You can make this dynamic if needed
	}
	description?: string
}

function fromHttpRoutePathToOpenAPIPath(httpRoutePath: string): [string, OpenAPIParameter[]] {
	const parameters: OpenAPIParameter[] = []
	const openapiPath = httpRoutePath.replace(/:([\w]+)/g, (_, paramName) => {
		parameters.push({
			name: paramName,
			in: 'path',
			required: true,
			schema: { type: 'string' },
			description: `Path parameter '${paramName}'`
		})
		return `{${paramName}}`
	})

	return [`/${openapiPath}`, parameters]
}

const okAsyncResponse = {
	200: {
		description: 'Job queued successfully',
		content: {
			'application/json': {
				schema: {
					type: 'string',
					format: 'uuid',
					example: 'f7641d4a-7890-4c83-9e8d-9d2d3bc5aeca'
				}
			}
		}
	}
}

const okSyncResponseDefault = {
	200: {
		description: 'Result of the script executed synchronously',
		content: {
			'application/json': {
				schema: {
					oneOf: [
						{ type: 'object', additionalProperties: true },
						{ type: 'array', items: {} },
						{ type: 'string' },
						{ type: 'number' },
						{ type: 'boolean' },
						{ type: 'null' }
					]
				},
				example: 'This could be any return value from the script'
			}
		}
	}
}
function generatePathObjectFromHttpRoutes(httpRoutes: HttpTrigger[]): OpenAPIV3_1.PathsObject {
	const paths = {}

	httpRoutes.forEach((httpRoute) => {
		const [openapiPath, parameters] = fromHttpRoutePathToOpenAPIPath(httpRoute.route_path)
		paths[openapiPath] = {
			...paths[openapiPath],
			[httpRoute.http_method]: {
				parameters,
				requestBody: {},
				respones: httpRoute.is_async ? okAsyncResponse : okSyncResponseDefault
			}
		}
	})

	return paths
}

export function generateOpenAPIspec(
	info: OpenAPIV3_1.InfoObject,
	servers: OpenAPIV3_1.ServerObject[],
	httpRoutes: HttpTrigger[]
): string {
	const paths = generatePathObjectFromHttpRoutes(httpRoutes)

	const spec = {
		info,
		servers,
		paths
	}

	return JSON.stringify(spec)
}
