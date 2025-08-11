import type { ChatCompletionTool } from 'openai/resources/index.mjs'
import type { Tool } from '../shared'
import { get } from 'svelte/store'
import { workspaceStore } from '$lib/stores'
import type { EndpointTool } from '$lib/gen/types.gen'
import { McpService } from '$lib/gen/services.gen'

function buildApiCallTool(endpointTool: EndpointTool): ChatCompletionTool {
	// Build the parameters schema for OpenAI function calling
	const parameters: Record<string, any> = {
		type: 'object',
		properties: {},
		required: []
	}

	// Add path parameters
	if (endpointTool.path_params_schema?.properties) {
		for (const [key, schema] of Object.entries(endpointTool.path_params_schema.properties)) {
			// Skip workspace parameter as it's auto-filled
			if (key === 'workspace') continue
			
			parameters.properties[key] = schema
			
			if (Array.isArray(endpointTool.path_params_schema.required) && endpointTool.path_params_schema.required.includes(key)) {
				parameters.required.push(key)
			}
		}
	}

	// Add query parameters
	if (endpointTool.query_params_schema?.properties) {
		for (const [key, schema] of Object.entries(endpointTool.query_params_schema.properties)) {
			parameters.properties[key] = schema
			
			if (Array.isArray(endpointTool.query_params_schema.required) && endpointTool.query_params_schema.required.includes(key)) {
				parameters.required.push(key)
			}
		}
	}

	// Add body parameters
	if (endpointTool.body_schema?.properties) {
		// For body params, we wrap them in a 'body' object
		parameters.properties.body = {
			type: 'object',
			description: 'Request body',
			properties: endpointTool.body_schema.properties,
			required: endpointTool.body_schema.required || []
		}
		
		if (Array.isArray(endpointTool.body_schema.required) && endpointTool.body_schema.required.length > 0) {
			parameters.required.push('body')
		}
	}

	return {
		type: 'function',
		function: {
			name: endpointTool.name,
			description: endpointTool.instructions || endpointTool.description,
			parameters
		}
	}
}

function buildToolsFromEndpoints(
	endpointTools: EndpointTool[]
): { tools: ChatCompletionTool[]; endpointMap: Record<string, { method: string; path: string }> } {
	const tools: ChatCompletionTool[] = []
	const endpointMap: Record<string, { method: string; path: string }> = {}

	for (const endpointTool of endpointTools) {
		const tool = buildApiCallTool(endpointTool)
		tools.push(tool)
		
		// Store the endpoint info in the map
		endpointMap[endpointTool.name] = {
			method: endpointTool.method,
			path: endpointTool.path
		}
	}

	return { tools, endpointMap }
}

export function createApiTools(
	chatTools: ChatCompletionTool[],
	endpointMap: Record<string, { method: string; path: string }> = {}
): Tool<{}>[] {
	return chatTools.map((chatTool) => {
		const toolName = chatTool.function.name
		const endpoint = endpointMap[toolName]
		const method = endpoint?.method?.toUpperCase() || 'GET'
		
		// Determine if tool needs confirmation based on method
		const needsConfirmation = ['DELETE', 'POST', 'PUT', 'PATCH'].includes(method)
		
		return {
			def: chatTool,
			requiresConfirmation: needsConfirmation,
			showDetails: true,
			fn: async ({ args, toolId, toolCallbacks }) => {
				const toolName = chatTool.function.name
				const endpoint = endpointMap[toolName]
				
				if (!endpoint) {
					throw new Error(`No endpoint mapping found for tool ${toolName}`)
				}

				try {
					const workspace = get(workspaceStore) as string
					let path = endpoint.path.replace('{workspace}', workspace)
					
					// Build URL with path parameters
					let url = `/api${path}`
					const queryParams: Record<string, string> = {}
					let requestBody: any = undefined

					// Process arguments
					for (const [key, value] of Object.entries(args)) {
						if (key === 'body') {
							requestBody = value
							continue
						}

						// Check if this is a path parameter
						if (url.includes(`{${key}}`)) {
							url = url.replace(`{${key}}`, encodeURIComponent(String(value)))
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

					toolCallbacks.setToolStatus(toolId, {
						content: `Calling ${toolName}...`,
					})

					const fetchOptions: RequestInit = {
						method: endpoint.method
					}

					// Add request body for POST/PUT/PATCH methods
					if (requestBody && ['POST', 'PUT', 'PATCH'].includes(endpoint.method.toUpperCase())) {
						fetchOptions.headers = {
							'Content-Type': 'application/json'
						}
						fetchOptions.body = JSON.stringify(requestBody)
					}

					const response = await fetch(url, fetchOptions)

					if (response.ok) {
						let result = ''
						if (response.headers.get('content-type')?.includes('application/json')) {
							result = await response.json()
						} else {
							result = await response.text()
						}
						const jsonResult = JSON.stringify({
							success: true,
							data: result
						})
						toolCallbacks.setToolStatus(toolId, {
							content: `Call to ${toolName} completed`,
							result: jsonResult,
						})
						return jsonResult
					} else {
						const text = await response.text()
						const jsonResult = JSON.stringify({
							success: false,
							error: text,
							status: response.status
						})
						toolCallbacks.setToolStatus(toolId, {
							content: `Call to ${toolName} failed`,
							result: jsonResult,
							error: `HTTP ${response.status}: ${text}`,
						})
						return jsonResult
					}
				} catch (error) {
					const errorMessage = `Error calling API: ${error instanceof Error ? error.message : String(error)}`
					toolCallbacks.setToolStatus(toolId, {
						content: `Call to ${toolName} failed`,
						error: errorMessage,
					})
					console.error(`Error calling API:`, error)
					return errorMessage
				}
			}
		}
	})
}

export async function loadApiTools(): Promise<Tool<{}>[]> {
	try {
		// Fetch the list of available MCP tools from the backend
		const endpointTools = await McpService.listMcpTools({
			workspace: get(workspaceStore) as string
		})
		
		// Build tools from the endpoint definitions
		const { tools: apiTools, endpointMap } = buildToolsFromEndpoints(endpointTools)
		
		// Create executable tools
		const executableApiTools = createApiTools(apiTools, endpointMap)
		return executableApiTools
	} catch (error) {
		console.error('Failed to load API tools:', error)
		return []
	}
}