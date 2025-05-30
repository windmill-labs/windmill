import { get, type Writable } from 'svelte/store'
import { page } from '$app/state'
import { getCompletion } from '$lib/components/copilot/lib'
import type {
	ChatCompletionChunk,
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionTool
} from 'openai/resources/index.mjs'
import { triggerablesByAI } from '$lib/stores'

// System prompt for the LLM
export const CHAT_SYSTEM_PROMPT = `
You are Windmill's intelligent assistant, designed to help users navigate the application and answer questions about its functionality. It is your only purpose to help the user in the context of the windmill application.
Windmill is an open-source developer platform for building internal tools, API integrations, background jobs, workflows, and user interfaces. It offers a unified system where scripts are automatically turned into sharable UIs and can be composed into flows or embedded in custom applications.

You have access to these tools:
1. View current buttons and inputs on the page (get_triggerable_components)
2. Execute buttons and inputs (trigger_component) 
3. Get documentation for user requests (get_documentation)

INSTRUCTIONS:
- When users ask about application features or concepts, first use get_documentation internally to retrieve accurate information about how to fulfill the user's request.
- Then check which page you are on using get_current_page_name. Then immediately use the available tools to guide the user through the application. Only wait for the user's confirmation if your are on a flow / script / app creation page. Do not wait for the user's confirmation before taking action on other pages.
- Use get_triggerable_components to understand available options, and then trigger the components using trigger_component. Then wait a moment before rescanning the current page, and then continue with the next step. Do this 5 times max.
- If you are not able to fulfill the user's request after 5 attempts, redirect the user to the documentation.

GENERAL PRINCIPLES:
- Be concise but thorough
- Focus on taking action and completing the user's goals
- Maintain a friendly, professional tone
- If you encounter an error or can't complete a request, explain why and suggest alternatives
- When asked about a specific script, flow or app, first check components directly related to the mentioned entity, before checking the other components.
- When you do not find what you are looking for on the current page, go to the home page by looking for the "Home" component, then scan the components again.

Always use the provided tools purposefully and appropriately to achieve the user's goals.
Your actions only allow you to navigate the application through the provided tools.
When you complete the user's request, do not say "I created..." or "I updated..." or "I deleted...", but rather say something like "Here is where you can find what you were looking for...". Complete your response with precisions about how it works based on the documentation. Also drop a link to the relevant documentation if possible.

Exemple of good behavior:
- User: "How can I set my AI providers?"
- You: <call get_documentation and fetch relevant documentation>
- You: <call get_triggerable_components to find relevant components>
- You: <trigger the components>
- You: "Here is where you can find what you were looking for. <precisions about the request based on the documentation>"
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
const GET_PAGE_HTML_TOOL: ChatCompletionTool = {
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
				}
			},
			required: ['id']
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

function getTriggerableComponents(): string {
	try {
		// Get components registered in the triggerablesByAI store
		const registeredComponents = get(triggerablesByAI)
		let result = 'TRIGGERABLE_COMPONENTS:\n'

		// If there are no components registered, return a message
		if (Object.keys(registeredComponents).length === 0) {
			return 'No AI-triggerable components are currently available on this page.\n'
		}

		// List each registered component with its ID and description
		Object.entries(registeredComponents).forEach(([id, component], index) => {
			result += `[${index}] ID: "${id}" - ${component.description} - Triggerable: ${component.onTrigger ? 'Yes' : 'No'}\n`
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

		const components = get(triggerablesByAI)
		const component = components[id]

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

async function getDocumentation(args: { request: string }): Promise<string | null> {
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
		return null
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

// Process tool calls from the LLM
async function processToolCall(
	toolCall: ChatCompletionMessageToolCall,
	messages: ChatCompletionMessageParam[]
) {
	try {
		const args = toolCall.function.arguments ? JSON.parse(toolCall.function.arguments) : {}
		let result = ''

		try {
			if (toolCall.function.name === 'get_triggerable_components') {
				result = getTriggerableComponents()
			} else if (toolCall.function.name === 'trigger_component') {
				result = triggerComponent(args)
			} else if (toolCall.function.name === 'get_documentation') {
				const docResult = await getDocumentation(args)
				result = docResult || 'No documentation found for this request'
			} else if (toolCall.function.name === 'get_current_page_name') {
				result = getCurrentPageName()
			} else {
				result = `Unknown tool: ${toolCall.function.name}`
			}
		} catch (err) {
			console.error(err)
			result = `Error while calling ${toolCall.function.name}: ${err.message}`
		}

		messages.push({
			role: 'tool',
			tool_call_id: toolCall.id,
			content: result
		})
	} catch (err) {
		console.error(err)
	}
}

// Main function to handle chat requests
export async function chatRequest(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	onNewToken: (token: string) => void
) {
	const toolDefs: ChatCompletionTool[] = [
		GET_PAGE_HTML_TOOL,
		EXECUTE_COMMAND_TOOL,
		GET_DOCUMENTATION_TOOL,
		GET_CURRENT_PAGE_NAME_TOOL
	]

	try {
		let completion: any = null

		while (true) {
			completion = await getCompletion(messages, abortController, toolDefs)

			if (completion) {
				const finalToolCalls: Record<number, ChatCompletionChunk.Choice.Delta.ToolCall> = {}

				for await (const chunk of completion) {
					if (!('choices' in chunk && chunk.choices.length > 0 && 'delta' in chunk.choices[0])) {
						continue
					}

					const c = chunk as ChatCompletionChunk
					const delta = c.choices[0].delta.content

					if (delta) {
						onNewToken(delta)
					}

					const toolCalls = c.choices[0].delta.tool_calls || []

					for (const toolCall of toolCalls) {
						const { index } = toolCall
						const finalToolCall = finalToolCalls[index]

						if (!finalToolCall) {
							finalToolCalls[index] = toolCall
						} else {
							if (toolCall.function?.arguments) {
								if (!finalToolCall.function) {
									finalToolCall.function = toolCall.function
								} else {
									finalToolCall.function.arguments =
										(finalToolCall.function.arguments ?? '') + toolCall.function.arguments
								}
							}
						}
					}
				}

				const toolCalls = Object.values(finalToolCalls).filter(
					(toolCall) => toolCall.id !== undefined && toolCall.function?.arguments !== undefined
				) as ChatCompletionMessageToolCall[]

				if (toolCalls.length > 0) {
					messages.push({
						role: 'assistant',
						tool_calls: toolCalls
					})

					for (const toolCall of toolCalls) {
						await processToolCall(toolCall, messages)
					}
				} else {
					break
				}
			}
		}

		return completion
	} catch (err) {
		if (!abortController.signal.aborted) {
			console.error(err)
			throw err
		}
	}
}

// Prepare initial system message
export function prepareSystemMessage(): ChatCompletionMessageParam {
	return {
		role: 'system',
		content: CHAT_SYSTEM_PROMPT
	}
}

// Interface for chat context
export interface AIChatContext {
	loading: Writable<boolean>
	currentReply: Writable<string>
}
