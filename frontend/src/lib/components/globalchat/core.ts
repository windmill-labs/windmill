import { get, type Writable } from 'svelte/store'
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
You are an assistant that can interact with the user's web page.
You have access to tools that let you:
1. View the current HTML of the page
2. Execute commands like clicking on elements

When asked to interact with the page:
- First examine the page structure to understand what's available
- Be precise when referencing elements
- Explain what you're doing before taking action
- Be helpful but cautious with actions that might change state
- After executing a command, wait for 1 second before rechecking the page and continuing fulfulling the user request. do this 3 times maximum.

Use the provided tools only when necessary and appropriate.
`

// Tool definitions
const GET_PAGE_HTML_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'get_page_html',
		description: 'Gets the current HTML structure of the page',
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
		name: 'execute_command',
		description: 'Executes a command to trigger an AI component',
		parameters: {
			type: 'object',
			properties: {
				id: {
					type: 'string',
					description: 'ID of the AI-triggerable component'
				}
			},
			required: ['id']
		}
	}
}

// Function to get page HTML
function getPageHtml(): string {
	try {
		// Get a clean version of the page HTML, removing scripts and other non-visible elements
		const html = document.documentElement.outerHTML

		// Process the HTML to extract useful information about interactive elements
		const interactiveElements = extractInteractiveElements()

		return `
PAGE_HTML_SUMMARY:
${interactiveElements}

FULL_HTML (truncated for readability):
${html.substring(0, 15000)}${html.length > 15000 ? '...(truncated)' : ''}
`
	} catch (error) {
		console.error('Error getting page HTML:', error)
		return 'Error getting page HTML: ' + error.message
	}
}

// Helper function to extract interactive elements from the page
function extractInteractiveElements(): string {
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
			result += `[${index}] ID: "${id}" - ${component.description}\n`
		})

		return result
	} catch (error) {
		console.error('Error extracting triggerable components:', error)
		return 'Error extracting triggerable components: ' + error.message
	}
}

// Function to execute commands on the page
function executeCommand(args: { id: string }): string {
	const { id } = args

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
			component.onTrigger(id)
			return `Successfully triggered component: ${id} (${component.description})`
		} else {
			return `Component ${id} has no trigger handler defined`
		}
	} catch (error) {
		console.error('Error executing command:', error)
		return `Error executing command: ${error.message}`
	}
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
			if (toolCall.function.name === 'get_page_html') {
				result = getPageHtml()
			} else if (toolCall.function.name === 'execute_command') {
				result = executeCommand(args)
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
	const toolDefs: ChatCompletionTool[] = [GET_PAGE_HTML_TOOL, EXECUTE_COMMAND_TOOL]

	try {
		let completion: any = null

		while (true) {
			completion = await getCompletion(messages, abortController, toolDefs)
			console.log(completion)

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

// Prepare user message with context
export function prepareUserMessage(message: string): string {
	return `
MESSAGE: ${message}

Feel free to use the get_page_html tool first if you need to understand the current page structure.
`
}

// Interface for chat context
export interface AIChatContext {
	loading: Writable<boolean>
	currentReply: Writable<string>
}
