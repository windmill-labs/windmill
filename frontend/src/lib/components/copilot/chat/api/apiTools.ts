import type { ChatCompletionTool } from 'openai/resources/index.mjs'
import type { Tool } from '../shared'
import { get } from 'svelte/store'
import { workspaceStore } from '$lib/stores'

// Type definitions for the EndpointTool from backend
interface EndpointTool {
	name: string
	description: string
	instructions: string
	path: string
	method: string
	path_params_schema?: any
	query_params_schema?: any
	body_schema?: any
}

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
			
			if (endpointTool.path_params_schema.required?.includes(key)) {
				parameters.required.push(key)
			}
		}
	}

	// Add query parameters
	if (endpointTool.query_params_schema?.properties) {
		for (const [key, schema] of Object.entries(endpointTool.query_params_schema.properties)) {
			parameters.properties[key] = schema
			
			if (endpointTool.query_params_schema.required?.includes(key)) {
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
		
		if (endpointTool.body_schema.required?.length > 0) {
			parameters.required.push('body')
		}
	}

	return {
		type: 'function',
		function: {
			name: 'api_' + endpointTool.name,
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
		endpointMap['api_' + endpointTool.name] = {
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
		return {
			def: chatTool,
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

					// Log the constructed URL
					console.log(`Calling API: ${endpoint.method} ${url} with args:`, args)

					toolCallbacks.setToolStatus(toolId, `Calling ${toolName.replace('api_', '')}...`, {
						toolName: toolName,
						description: chatTool.function.description,
						parameters: args,
						isLoading: true,
						startedAt: Date.now()
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

					console.log('fetchOptions', fetchOptions)

					// add fake delay
					await new Promise((resolve) => setTimeout(resolve, 5000))

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
						toolCallbacks.setToolStatus(toolId, `Call to ${toolName.replace('api_', '')} completed`, {
							toolName: toolName,
							description: chatTool.function.description,
							parameters: args,
							result: jsonResult,
							isLoading: false,
							completedAt: Date.now()
						})
						return jsonResult
					} else {
						const text = await response.text()
						const jsonResult = JSON.stringify({
							success: false,
							error: text,
							status: response.status
						})
						toolCallbacks.setToolStatus(toolId, `Call to ${toolName.replace('api_', '')} failed`, {
							toolName: toolName,
							description: chatTool.function.description,
							parameters: args,
							result: jsonResult,
							error: `HTTP ${response.status}: ${text}`,
							isLoading: false,
							completedAt: Date.now()
						})
						return jsonResult
					}
				} catch (error) {
					const errorMessage = `Error calling API: ${error instanceof Error ? error.message : String(error)}`
					toolCallbacks.setToolStatus(toolId, `Call to ${toolName.replace('api_', '')} failed`, {
						toolName: toolName,
						description: chatTool.function.description,
						parameters: args,
						error: errorMessage,
						isLoading: false,
						completedAt: Date.now()
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
		const response = await fetch(`/api/mcp/w/${get(workspaceStore)}/list_tools`)
		
		if (!response.ok) {
			throw new Error(`Failed to fetch MCP tools: ${response.status} ${response.statusText}`)
		}
		
		const endpointTools: EndpointTool[] = await response.json()
		console.log('Loaded MCP tools:', endpointTools.length, 'tools')
		
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