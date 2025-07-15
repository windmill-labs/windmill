import type { ChatCompletionTool } from 'openai/resources/index.mjs'
import type { Tool } from '../shared'
import { get } from 'svelte/store'
import { workspaceStore } from '$lib/stores'

// OpenAPI type definitions
interface OpenAPIParameter {
	name: string
	in: string
	description?: string
	required?: boolean
	schema?: {
		type?: string
		format?: string
	}
}

interface OpenAPIRequestBody {
	description?: string
	required?: boolean
	content: {
		[contentType: string]: {
			schema: {
				type?: string
				properties?: Record<string, any>
			}
		}
	}
}

interface OpenAPIOperation {
	operationId?: string
	summary?: string
	description?: string
	parameters?: OpenAPIParameter[]
	requestBody?: OpenAPIRequestBody
	responses?: Record<string, any>
	tags?: string[]
}

interface OpenAPIPathItem {
	get?: OpenAPIOperation
	post?: OpenAPIOperation
	put?: OpenAPIOperation
	delete?: OpenAPIOperation
	patch?: OpenAPIOperation
	options?: OpenAPIOperation
	parameters?: OpenAPIParameter[]
	summary?: string
	description?: string
}

interface OpenAPISpec {
	paths: {
		[path: string]: OpenAPIPathItem
	}
}

const buildApiCallTools = (
	name: string,
	description: string,
	parameters: any
): ChatCompletionTool => {
	return {
		type: 'function',
		function: {
			name,
			description,
			parameters
		}
	}
}

export function buildToolsFromOpenApi(
	openApiSpec: OpenAPISpec,
	options: {
		pathFilter?: (path: string) => boolean
		operationFilter?: (operation: OpenAPIOperation) => boolean
		methodFilter?: string[]
	} = {}
): { tools: ChatCompletionTool[]; endpointMap: Record<string, string> } {
	const tools: ChatCompletionTool[] = []
	const endpointMap: Record<string, string> = {}
	const { pathFilter, methodFilter = ['get', 'post', 'put', 'delete', 'patch'] } = options

	// Iterate through all paths in the OpenAPI spec
	for (const [path, pathItem] of Object.entries(openApiSpec.paths)) {
		if (pathFilter && !pathFilter(path)) continue

		for (const [method, operation] of Object.entries(pathItem)) {
			// Skip non-operation properties
			if (
				method === 'parameters' ||
				method === 'servers' ||
				method === 'summary' ||
				method === 'description'
			)
				continue

			// Skip methods not in methodFilter
			if (!methodFilter.includes(method.toLowerCase())) continue

			// Type cast to OpenAPIOperation
			const op = operation as OpenAPIOperation
			if (!op.operationId || !op.summary) {
				console.error(`Operation ${method} ${path} has no operationId or summary`)
				continue
			}

			// Build the parameters schema
			const parameters: Record<string, any> = {
				type: 'object',
				properties: {},
				required: []
			}

			// Process path parameters
			const pathParams = [...(pathItem.parameters || []), ...(op.parameters || [])].filter(
				(p: OpenAPIParameter) => p.in === 'path'
			)

			// Process query parameters
			const queryParams = (op.parameters || []).filter((p: OpenAPIParameter) => p.in === 'query')

			// Add path parameters
			for (const param of pathParams) {
				parameters.properties[param.name] = {
					type: param.schema?.type || 'string',
					description: param.description || `Path parameter: ${param.name}`
				}

				if (param.required) {
					parameters.required.push(param.name)
				}
			}

			// Add query parameters
			for (const param of queryParams) {
				parameters.properties[param.name] = {
					type: param.schema?.type || 'string',
					description: param.description || `Query parameter: ${param.name}`
				}

				if (param.required) {
					parameters.required.push(param.name)
				}
			}

			// Handle request body if present
			if (op.requestBody) {
				const contentType = Object.keys(op.requestBody.content || {})[0]
				if (contentType) {
					const schema = op.requestBody.content[contentType].schema

					if (schema) {
						parameters.properties.body = {
							type: 'object',
							description: op.requestBody.description || 'Request body',
							properties: schema.properties || {}
						}

						if (op.requestBody.required) {
							parameters.required.push('body')
						}
					}
				}
			}

			const tool = buildApiCallTools(
				'api_' + op.operationId,
				op.summary || op.description || `${method.toUpperCase()} ${path}`,
				parameters
			)

			// Store the endpoint path in the map
			endpointMap['api_' + op.operationId] = `${method.toUpperCase()} ${path}`

			tools.push(tool)
		}
	}

	return { tools, endpointMap }
}

export function createApiTools(
	chatTools: ChatCompletionTool[],
	endpointMap: Record<string, string> = {}
): Tool<{}>[] {
	return chatTools.map((chatTool) => {
		return {
			def: chatTool,
			fn: async ({ args, toolId, toolCallbacks }) => {
				const toolName = chatTool.function.name
				let endpoint = endpointMap[toolName] || ''
				endpoint = endpoint.replace('{workspace}', get(workspaceStore) as string)
				toolCallbacks.setToolStatus(toolId, `Calling API endpoint (${endpoint})...`)

				try {
					// Extract method and path from endpoint
					const [method, path] = endpoint.split(' ', 2)

					if (!endpoint || !method || !path) {
						throw new Error(`Invalid endpoint for tool ${toolName}: ${endpoint}`)
					}

					// Build URL with path parameters
					let url = `/api${path}`
					const queryParams: Record<string, string> = {}

					// Process arguments
					for (const [key, value] of Object.entries(args)) {
						if (key === 'body') continue // Body is handled separately

						// Check if this is a path parameter
						if (url.includes(`:${key}`)) {
							url = url.replace(`:${key}`, encodeURIComponent(String(value)))
						} else {
							// Assume it's a query parameter
							queryParams[key] = String(value)
						}
					}
					// Add query parameters to URL if needed
					if (Object.keys(queryParams).length > 0) {
						const searchParams = new URLSearchParams()
						for (const [key, value] of Object.entries(queryParams)) {
							searchParams.append(key, value)
						}
						url += `?${searchParams.toString()}`
					}

					// Log the constructed URL
					console.log(`Calling API: ${method} ${url} with args: ${JSON.stringify(args)}`)

					const response = await fetch(url, {
						method: method
					})
					const data = await response.json()

					// For now, return a placeholder response
					// In a real implementation, we would make the actual API call here
					toolCallbacks.setToolStatus(toolId, `API call to ${endpoint} completed`)
					return JSON.stringify({
						success: true,
						data: data
					})
				} catch (error) {
					toolCallbacks.setToolStatus(toolId, `API call to ${endpoint} failed`)
					console.error(`Error calling API to ${endpoint}:`, error)
					return `Error calling API: ${error instanceof Error ? error.message : String(error)}`
				}
			}
		}
	})
}

export async function loadApiTools(): Promise<Tool<{}>[]> {
	try {
		const response = await fetch('/api/openapi.json')
		const openApiSpec = (await response.json()) as OpenAPISpec
		const pathsToInclude = [
			'jobs',
			'scripts',
			'flows',
			'resources',
			'variables',
			'schedules',
			'workers',
			'srch'
		]

		const { tools: apiTools, endpointMap } = buildToolsFromOpenApi(openApiSpec, {
			pathFilter: (path) => pathsToInclude.some((p) => path.includes(`/${p}/`)),
			methodFilter: ['get']
		})

		const executableApiTools = createApiTools(apiTools, endpointMap)
		return executableApiTools
	} catch (error) {
		console.error('Failed to load API tools:', error)
		return []
	}
}
