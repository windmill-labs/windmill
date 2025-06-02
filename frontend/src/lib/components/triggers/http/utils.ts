import { base } from '$lib/base'
import { isCloudHosted } from '$lib/cloud'
import { random_adj } from '$lib/components/random_positive_adjetive'
import type { HttpMethod, NewHttpTrigger } from '$lib/gen'
import { HttpTriggerService } from '$lib/gen/services.gen'
import { sendUserToast } from '$lib/toast'
import type { OpenAPI, OpenAPIV2, OpenAPIV3, OpenAPIV3_1 } from 'openapi-types'
import type { Writable } from 'svelte/store'
import { get } from 'svelte/store'
import YAML from 'yaml'

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
		raw_string: routeCfg.raw_string
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

export type Source = 'OpenApiFile' | 'OpenApi' | 'OpenApiURL'

function processV2(doc: OpenAPIV2.Document): NewHttpTrigger[] {
	const paths = doc.paths

	const httpTrigger: NewHttpTrigger[] = []

	for (const path in paths) {
		const pathItem = paths[path]
		if (!pathItem) continue

		const methods: HttpMethod[] = ['get', 'post', 'put', 'patch', 'delete']

		for (const method of methods) {
			const routeDetail = pathItem[method]
			if (!routeDetail) continue

			httpTrigger.push({
				route_path: path.replace(/^\/+/, ''),
				http_method: method,
				authentication_method: 'none',
				workspaced_route: false,
				is_async: true,
				script_path: '',
				raw_string: false,
				is_flow: false,
				is_static_website: false,
				wrap_body: false,
				path: `f/test/${random_adj()}`
			})
		}
	}

	return httpTrigger
}

function processV3(doc: OpenAPIV3.Document): NewHttpTrigger[] {
	const paths = doc.paths

	const httpTrigger: NewHttpTrigger[] = []

	for (const path in paths) {
		const pathItem = paths[path]
		if (!pathItem) continue

		const methods: HttpMethod[] = ['get', 'post', 'put', 'patch', 'delete']

		for (const method of methods) {
			const routeDetail = pathItem[method]
			if (!routeDetail) continue

			httpTrigger.push({
				route_path: path.replace(/^\/+/, ''),
				http_method: method,
				authentication_method: 'none',
				workspaced_route: false,
				is_async: true,
				script_path: '',
				raw_string: false,
				is_flow: false,
				is_static_website: false,
				wrap_body: false,
				path: `f/test/${random_adj()}`
			})
		}
	}
	return httpTrigger
}

function processV3_1(doc: OpenAPIV3_1.Document): NewHttpTrigger[] {
	const paths = doc.paths

	const httpTrigger: NewHttpTrigger[] = []

	for (const path in paths) {
		const pathItem = paths[path]
		if (!pathItem) continue

		const methods: HttpMethod[] = ['get', 'post', 'put', 'patch', 'delete']

		for (const method of methods) {
			const routeDetail = pathItem[method]
			if (!routeDetail) continue

			httpTrigger.push({
				route_path: path.replace(/^\/+/, ''),
				http_method: method,
				authentication_method: 'none',
				workspaced_route: false,
				is_async: true,
				script_path: '',
				raw_string: false,
				is_flow: false,
				is_static_website: false,
				wrap_body: false,
				path: `f/test/${random_adj()}`
			})
		}
	}

	return httpTrigger
}

export function isV2(doc: OpenAPI.Document): doc is OpenAPIV2.Document {
	return 'swagger' in doc && doc.swagger === '2.0'
}

export function isV3(doc: OpenAPI.Document): doc is OpenAPIV3.Document {
	return 'openapi' in doc && typeof doc.openapi === 'string' && doc.openapi.startsWith('3.0')
}

export function isV3_1(doc: OpenAPI.Document): doc is OpenAPIV3_1.Document {
	return 'openapi' in doc && typeof doc.openapi === 'string' && doc.openapi.startsWith('3.1')
}

export async function generateHttpTriggerFromOpenApi(
	openApiSpec: string
): Promise<NewHttpTrigger[]> {
	try {
		let data: any

		if (openApiSpec.trimStart().at(0) === '{') {
			data = JSON.parse(openApiSpec)
		} else {
			data = YAML.parse(openApiSpec)
		}

		const swaggerParser = (await import('@apidevtools/swagger-parser')).default

		const document = await swaggerParser.validate(data)
		if (isV2(document)) {
			return processV2(document)
		} else if (isV3(document)) {
			return processV3(document)
		} else {
			return processV3_1(document)
		}
	} catch (error) {
		throw error
	}
}
