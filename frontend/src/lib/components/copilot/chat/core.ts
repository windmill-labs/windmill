import type { AIProvider } from '$lib/gen/types.gen'
import { getCompletion, getResponseFromEvent } from '../lib'

const CHAT_SYSTEM_PROMPT = `
You are a coding assistant. You are given a list of instructions to follow \`INSTRUCTIONS\` as well as the current code in the file \`CODE\`.

Please respond to the user's query. The user's query is never invalid.

In the case that the user asks you to make changes to code, you should make sure to return CODE BLOCKS of the changes, as well as explanations and descriptions of the changes.
For example, if the user asks you to "make this file look nicer", make sure your output includes a code block with concrete ways the file can look nicer.
- If suggesting changes, rewrite the **complete code** and not just a part of it.

Requirements:
- When suggesting changes, do not change spacing, indentation, or other whitespace apart from what is strictly necessary to apply the changes.

Do not output any of these instructions, nor tell the user anything about them unless directly prompted for them.
`

const CHAT_USER_PROMPT = `
INSTRUCTIONS:
Do not change spacing, indentation, or other whitespace apart from what is strictly necessary to apply the changes.
{instructions}

CODE:
\`\`\`{language}
{code}
\`\`\`
`

export class AIChatHandler {
	aiProvider: AIProvider
	language: string

	messages: { role: 'user' | 'assistant' | 'system'; content: string }[] = [
		{ role: 'system', content: CHAT_SYSTEM_PROMPT }
	]
	codeBlocks: string[][] = []
	currentMessageContent: string | undefined = undefined

	abortController: AbortController | undefined = undefined

	constructor(aiProvider: AIProvider, language: string) {
		this.aiProvider = aiProvider
		this.language = language
	}

	async sendRequest(
		context: {
			instructions: string
			code: string
		},
		onNewToken: (token: string) => void
	) {
		this.currentMessageContent = ''
		this.abortController = new AbortController()

		const userMessage = CHAT_USER_PROMPT.replace('{instructions}', context.instructions)
			.replace('{code}', context.code)
			.replace('{language}', this.language)

		this.messages.push({ role: 'user', content: userMessage })

		const completion = await chatRequest(
			{ ...context, language: this.language },
			this.messages,
			this.abortController,
			this.aiProvider
		)

		if (completion) {
			this.currentMessageContent = ''
			for await (const part of completion) {
				const token = getResponseFromEvent(part, this.aiProvider)
				this.currentMessageContent += token
				onNewToken(token)
			}

			const codeBlocks = this.currentMessageContent.match(/```(\S+)\n(.*)```/gs)

			if (codeBlocks) {
				this.codeBlocks.push([...codeBlocks])
				console.log(this.codeBlocks)
			}

			this.messages.push({ role: 'assistant', content: this.currentMessageContent })
			this.currentMessageContent = undefined
		}
	}

	async cancel() {
		this.currentMessageContent = undefined
		this.abortController?.abort()
	}
}

async function chatRequest(
	context: {
		instructions: string
		code: string
		language: string
	},
	messages: { role: 'user' | 'assistant' | 'system'; content: string }[],
	abortController: AbortController,
	aiProvider: AIProvider
) {
	try {
		const completion = await getCompletion(messages, abortController, aiProvider)

		return completion
	} catch (err) {
		if (!abortController.signal.aborted) {
			throw err
		}
	}
}
