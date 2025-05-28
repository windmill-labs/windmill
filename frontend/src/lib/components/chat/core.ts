import { get, type Writable } from 'svelte/store'
import { getCompletion } from '$lib/components/copilot/lib'
import type {
	ChatCompletionChunk,
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionTool
} from 'openai/resources/index.mjs'
import { triggerablesByAI } from '$lib/stores'
import OpenAI from 'openai'

// System prompt for the LLM
export const CHAT_SYSTEM_PROMPT = `
You are Windmill's intelligent assistant, designed to help users navigate the application and answer questions about its functionality.

You have access to these tools:
1. View current triggerable components on the page (get_triggerable_components)
2. Execute component trigger functions (trigger_component) 
3. Get documentation for user requests (get_documentation)

RESPONDING TO QUESTIONS:
- When users ask about application features or concepts, use get_documentation to retrieve accurate information
- Present information concisely and clearly, highlighting key points
- For complex topics, offer to guide users through relevant sections of the application

NAVIGATION ASSISTANCE:
- When users want to perform an action, first use get_triggerable_components to understand available options
- Always explain what you'll do before taking any action
- Take action only when you're confident it matches the user's intent
- For multi-step processes:
  * Guide users step-by-step, explaining each action
  * After each action, wait briefly then recheck available components before continuing
  * Maintain context throughout the interaction

USER EXPERIENCE GUIDELINES:
- Be proactive in suggesting helpful next steps
- If a request is ambiguous, ask clarifying questions before taking action
- After completing a task involving panels or drawers, look for and use close/dismiss buttons
- If you encounter an error or can't complete a request, explain why and suggest alternatives
- Adapt your level of guidance based on user expertise (more detailed for beginners)

GENERAL PRINCIPLES:
- Be concise but thorough
- Focus on being helpful rather than just informative
- Maintain a friendly, professional tone
- Remember user preferences between interactions when possible

Always use the provided tools purposefully and appropriately to achieve the user's goals.
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
			result += `[${index}] ID: "${id}" - ${component.description}\n`
		})

		return result
	} catch (error) {
		console.error('Error getting triggerable components:', error)
		return 'Error getting triggerable components: ' + error.message
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
	const client = new OpenAI({
		apiKey: '',
		baseURL: 'https://api.inkeep.com/v1',
		dangerouslyAllowBrowser: true
	})
	const retrieval = await client.chat.completions.create({
		model: 'inkeep-rag',
		messages: [{ role: 'user', content: args.request }]
	})
	if (!retrieval.choices[0].message.content) {
		return null
	}

	// Parse the raw response
	const raw = retrieval.choices[0].message.content
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

	return retrieval.choices[0].message.content
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
		GET_DOCUMENTATION_TOOL
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
