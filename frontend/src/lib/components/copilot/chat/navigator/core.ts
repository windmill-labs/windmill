import { page } from '$app/state'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionTool,
	ChatCompletionUserMessageParam
} from 'openai/resources/index.mjs'
import type { Tool } from '../shared'
import { ResourceService } from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { get } from 'svelte/store'
import { triggerablesByAi } from '../sharedChatState.svelte'

export const CHAT_SYSTEM_PROMPT = `
You are Windmill's intelligent assistant, designed to help users navigate the application and answer questions about its functionality. It is your only purpose to help the user in the context of the windmill application.
Windmill is an open-source developer platform for building internal tools, API integrations, background jobs, workflows, and user interfaces. It offers a unified system where scripts are automatically turned into sharable UIs and can be composed into flows or embedded in custom applications.

You have access to these tools:
1. View current buttons and inputs on the page (get_triggerable_components)
2. Execute buttons and inputs (trigger_component) 
3. Get documentation for user requests (get_documentation)
4. A list of tools to interact with the backend API

INSTRUCTIONS:
- When users ask about application features or concepts, first use get_documentation internally to retrieve accurate information about how to fulfill the user's request.
- Then immediately use the available tools to guide the user through the application. Do not wait for the user's confirmation before taking action.
- If you detect a confirmation modal that needs user confirmation, stop the navigation and let the user know that the action is pending confirmation.
- Use get_triggerable_components to understand available options, and then trigger the components using trigger_component. Then wait a moment before rescanning the current page, and then continue with the next step. Do this 5 times max.
- Make sure you navigated as far as possible before responding to the user. Always use get_triggerable_components one last time to make sure you didn't miss anything.
- If you are not able to fulfill the user's request after 5 attempts, redirect the user to the documentation.
- If you are asked to fill a form or act on an input, input the existing json object and change the fields the user asked you to change. Take into account the prompt_for_ai field of the schema to know what and how to do changes. Then tell the user that you have updated the form, and ask him to review the changes before running the script or flow.
- For form inputs where format starts with "resource-" and is not "resource-obj", fetch the available resources using get_available_resources, and then use the resource_path prefixed with "$res:" to fill the input.
- If you are not sure about an input, set the ones you are sure about, and then ask the user for the value of the input you are not sure about.

GENERAL PRINCIPLES:
- Be concise but thorough
- Focus on taking action and completing the user's goals
- Maintain a friendly, professional tone
- If you encounter an error or can't complete a request, explain why and suggest alternatives
- When asked about a specific script, flow or app, first check components directly related to the mentioned entity, before checking the other components.
- When you do not find what you are looking for on the current page, go to the home page by looking for the "Home" component, then scan the components again.

IMPORTANT CONSIDERATIONS:
- If you do an API call, make sure you ask the user if he also wants you to navigate the application to fulfill his request.
- The user might have changed the page in the middle of the conversation, so make sure you rescan the page on each user request instead of just responding that you cannot find what the user is asking for.
- If you navigate to a script creation page, consider this:
  - The page opens with the settings drawer open. After doing the changes mentioned by the user, close the settings drawer.
  - Then if the user has described what he wanted the script to do, switch to script mode with the change_mode tool, and use the new tools you'll have access to to edit the script.
- If you navigate to a flow creation page, consider this:
  - If the user has described what he wanted the flow to do, switch to flow mode with the change_mode tool before using the new tools you'll have access to to edit the flow.

API_TOOLS_RESTRICTIONS:
- You can only use the API tools to fetch data from the backend API after you tried to navigate the application to fulfill the user's request and it's not enough to do so. ALWAYS ask the user if he also wants you to navigate the application to fulfill his request.
- If you use api tools, also fetch the relevant documentation to help the user understand the data you fetched, with a link to the documentation if possible.

RETRIEVE_AVAILABLE_RESOURCES_RESTRICTION:
- You can only use the get_available_resources tool to fill a form or an input based on the user's request. Do not use it when directly asked to fetch available resources, use the API tools instead.

Always use the provided tools purposefully and appropriately to achieve the user's goals.
Your actions only allow you to navigate the application through the provided tools.
When you complete the user's request, do not say "I created..." or "I updated..." or "I deleted...", but rather complete your response with precisions about how it works based on the documentation. Also drop a link to the relevant documentation if possible.

Example of good behavior:
- User: "How can I set my AI providers?"
- You: <call get_documentation and fetch relevant documentation>
- You: <call get_triggerable_components to find relevant components>
- You: <trigger the components>
- You: "<precisions about the request based on the documentation>"
`

const GET_DOCUMENTATION_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'get_documentation',
		description: 'Get the documentation for the user request',
		parameters: {
			type: 'object',
			properties: {
				request: {
					type: 'string',
					description: 'The user request'
				}
			},
			required: ['request']
		}
	}
}

// Tool definitions
const GET_TRIGGERABLE_COMPONENTS_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'get_triggerable_components',
		description: 'Get the current triggerable components on the page',
		parameters: {
			type: 'object',
			properties: {},
			required: []
		}
	}
}

const EXECUTE_COMMAND_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'trigger_component',
		description: 'Trigger a triggerable component',
		parameters: {
			type: 'object',
			properties: {
				id: {
					type: 'string',
					description: 'ID of the AI-triggerable component'
				},
				value: {
					type: 'string',
					description: 'Value to pass to the AI-triggerable component trigger function'
				},
				actionTaken: {
					type: 'string',
					description:
						'Short description of the action taken. Can be clicked, filled, etc. Includes which component was triggered.'
				}
			},
			required: ['id', 'actionTaken']
		}
	}
}

const GET_CURRENT_PAGE_NAME_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'get_current_page_name',
		description: 'Get the name of the current page the user is on.',
		parameters: {
			type: 'object',
			properties: {},
			required: []
		}
	}
}

const GET_AVAILABLE_RESOURCES_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'get_available_resources',
		description:
			"Get the available resources to the user. Only use this tool to fill a form or an input based on the user's request.",
		parameters: {
			type: 'object',
			properties: {
				resource_type: {
					type: 'string',
					description: 'The type of resource to get, separated by ","'
				}
			},
			required: ['resource_type']
		}
	}
}

function getTriggerableComponents(): string {
	try {
		// Get components registered in the triggerablesByAi store
		const registeredComponents = triggerablesByAi
		let result = 'TRIGGERABLE_COMPONENTS:\n'

		// If there are no components registered, return a message
		if (Object.keys(registeredComponents).length === 0) {
			return 'No AI-triggerable components are currently available on this page.\n'
		}

		// List each registered component with its ID and description
		Object.entries(registeredComponents).forEach(([id, component], index) => {
			result += `
				[${index}]
				ID: "${id}"
				Description: ${component.description}
				Triggerable: ${component.onTrigger ? 'Yes' : 'No'}
				\n`
		})

		return result
	} catch (error) {
		console.error('Error getting triggerable components:', error)
		return 'Error getting triggerable components: ' + error.message
	}
}

// Function to get the current page name
function getCurrentPageName(): string {
	try {
		const currentPage = page.url.pathname
		switch (currentPage) {
			case '/':
				return 'Home Page'
			case '/flows/add':
				return 'Flow creation page'
			case '/scripts/add':
				return 'Script creation page'
			case '/apps/add':
				return 'App creation page'
			default:
				return 'Non-specific page'
		}
	} catch (error) {
		console.error('Error getting current page name:', error)
		return 'Error getting current page name: ' + error.message
	}
}

// Function to execute commands on the page
function triggerComponent(args: { id: string; value: string }): string {
	const { id, value } = args

	try {
		// Handle triggering AI components
		if (!id) {
			return 'Trigger command requires an id parameter'
		}

		const component = triggerablesByAi[id]

		if (!component) {
			return `No triggerable component found with id: ${id}`
		}

		if (component.onTrigger) {
			component.onTrigger(value)
			return `Successfully triggered component: ${id} (${component.description})`
		} else {
			return `Component ${id} has no trigger handler defined`
		}
	} catch (error) {
		console.error('Error executing command:', error)
		return `Error executing command: ${error.message}`
	}
}

async function getDocumentation(args: { request: string }): Promise<string> {
	const retrieval = await fetch('/api/inkeep', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({
			model: 'inkeep-rag',
			messages: [{ role: 'user', content: args.request }],
			response_format: {
				type: 'json_object'
			}
		})
	})
	const data = await retrieval.json()
	if (!data.choices?.[0]?.message?.content) {
		return 'No documentation found for this request'
	}

	// Parse the raw response
	const raw = data.choices[0].message.content
	const parsed = JSON.parse(raw)

	// Clean up the response to include only essential information
	if (parsed.content && Array.isArray(parsed.content)) {
		const cleanedContent = parsed.content.map((item: any) => ({
			title: item.title,
			url: item.url,
			content: item.source?.content.map((c: any) => c.text).join('\n') || []
		}))
		// Limit the response to 30000 characters max
		const stringified = JSON.stringify({ content: cleanedContent }).slice(0, 30000)

		return stringified
	}

	return data.choices[0].message.content
}

async function getAvailableResources(args: { resource_type: string }): Promise<string> {
	const resources = await ResourceService.listResource({
		workspace: get(workspaceStore) as string,
		resourceType: args.resource_type
	})
	return JSON.stringify(resources)
}

const triggerComponentTool: Tool<{}> = {
	def: EXECUTE_COMMAND_TOOL,
	fn: async ({ args, toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, 'Triggering component...')
		const result = triggerComponent(args)
		toolCallbacks.setToolStatus(
			toolId,
			args.actionTaken.charAt(0).toUpperCase() + args.actionTaken.slice(1)
		)
		return result
	}
}

const getTriggerableComponentsTool: Tool<{}> = {
	def: GET_TRIGGERABLE_COMPONENTS_TOOL,
	fn: async ({ toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, 'Scanning the page...')
		const components = getTriggerableComponents()
		toolCallbacks.setToolStatus(toolId, 'Scanned the page')
		return components
	}
}

const getCurrentPageNameTool: Tool<{}> = {
	def: GET_CURRENT_PAGE_NAME_TOOL,
	fn: async ({ toolId, toolCallbacks }) => {
		const pageName = getCurrentPageName()
		toolCallbacks.setToolStatus(toolId, 'Retrieved current page name')
		return pageName
	}
}

export const getDocumentationTool: Tool<{}> = {
	def: GET_DOCUMENTATION_TOOL,
	fn: async ({ args, toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, 'Getting documentation...')
		try {
			const docResult = await getDocumentation(args)
			toolCallbacks.setToolStatus(toolId, 'Retrieved documentation')
			return docResult
		} catch (error) {
			toolCallbacks.setToolStatus(toolId, 'Error getting documentation')
			console.error('Error getting documentation:', error)
			return 'Failed to get documentation, pursuing with the user request...'
		}
	}
}

const getAvailableResourcesTool: Tool<{}> = {
	def: GET_AVAILABLE_RESOURCES_TOOL,
	fn: async ({ args, toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, 'Getting available resources...')
		try {
			const resources = await getAvailableResources(args)
			toolCallbacks.setToolStatus(toolId, 'Retrieved available resources')
			return resources
		} catch (error) {
			toolCallbacks.setToolStatus(toolId, 'Error getting available resources')
			console.error('Error getting available resources:', error)
			return 'Failed to get available resources, pursuing with the user request...'
		}
	}
}

export const navigatorTools: Tool<{}>[] = [
	getTriggerableComponentsTool,
	triggerComponentTool,
	getDocumentationTool,
	getCurrentPageNameTool,
	getAvailableResourcesTool
]

export function prepareNavigatorSystemMessage(): ChatCompletionSystemMessageParam {
	return {
		role: 'system',
		content: CHAT_SYSTEM_PROMPT
	}
}

export function prepareNavigatorUserMessage(instructions: string): ChatCompletionUserMessageParam {
	return {
		role: 'user',
		content: instructions
	}
}
