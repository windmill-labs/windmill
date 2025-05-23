import { get, type Writable } from 'svelte/store'
import { getCompletion } from '$lib/components/copilot/lib'
import type {
	ChatCompletionChunk,
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionTool
} from 'openai/resources/index.mjs'
import { userStore } from '$lib/stores'

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
		description: 'Executes a command on the page, such as clicking a button',
		parameters: {
			type: 'object',
			properties: {
				type: {
					type: 'string',
					description: 'The type of command (click, input, select, etc.)',
					enum: ['click', 'input', 'select']
				},
				selector: {
					type: 'string',
					description: 'CSS selector or XPath to identify the element'
				},
				value: {
					type: 'string',
					description: 'Value to use for input or select commands (optional)'
				}
			},
			required: ['type', 'selector']
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
		const elements = document.querySelectorAll('button, a, input, select, [role="button"]')
		let result = 'INTERACTIVE_ELEMENTS:\n'

		elements.forEach((el, index) => {
			const tagName = el.tagName.toLowerCase()
			const text = el.textContent?.trim() || ''
			const id = el.id ? `id="${el.id}"` : ''
			const classes = Array.from(el.classList).join(' ')
			const classAttr = classes ? `class="${classes}"` : ''
			const type = el.getAttribute('type') || ''
			const typeAttr = type ? `type="${type}"` : ''
			const value = (el as HTMLInputElement).value || ''
			const valueAttr = value ? `value="${value}"` : ''
			const placeholder = el.getAttribute('placeholder') || ''
			const placeholderAttr = placeholder ? `placeholder="${placeholder}"` : ''
			const href = (el as HTMLAnchorElement).href || ''
			const hrefAttr = href ? `href="${href}"` : ''
			const role = el.getAttribute('role') || ''
			const roleAttr = role ? `role="${role}"` : ''

			// Create a basic selector for this element
			const selector = el.id
				? `#${el.id}`
				: tagName + (classes ? `.${classes.replace(/\s+/g, '.')}` : '')

			result += `[${index}] <${tagName} ${id} ${classAttr} ${typeAttr} ${valueAttr} ${placeholderAttr} ${hrefAttr} ${roleAttr}>${text}</${tagName}> (selector: ${selector})\n`
		})

		return result
	} catch (error) {
		console.error('Error extracting interactive elements:', error)
		return 'Error extracting interactive elements: ' + error.message
	}
}

// Function to execute commands on the page
function executeCommand(args: any): string {
	const { type, selector, value } = args

	try {
		const element = document.querySelector(selector)
		if (!element) {
			return `No element found matching selector: ${selector}`
		}

		switch (type) {
			case 'click':
				;(element as HTMLElement).click()
				return `Successfully clicked element: ${selector}`

			case 'input':
				if (!value) {
					return 'Input command requires a value'
				}
				const inputElement = element as HTMLInputElement
				inputElement.value = value
				inputElement.dispatchEvent(new Event('input', { bubbles: true }))
				inputElement.dispatchEvent(new Event('change', { bubbles: true }))
				return `Successfully set input value for element: ${selector}`

			case 'select':
				if (!value) {
					return 'Select command requires a value'
				}
				const selectElement = element as HTMLSelectElement
				selectElement.value = value
				selectElement.dispatchEvent(new Event('change', { bubbles: true }))
				return `Successfully set select value for element: ${selector}`

			default:
				return `Unsupported command type: ${type}`
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
