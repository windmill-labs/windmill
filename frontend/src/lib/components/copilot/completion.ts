import type { ChatCompletionMessageParam } from 'openai/resources/chat'
import { getNonStreamingCompletion } from './lib'
import { codeCompletionLoading } from '$lib/stores'

const systemPrompt = `You are a code completion assistant, return the code that should go instead of the <completion_tokens>.

Only return the completion tokens. Wrap the completion tokens in a code block (\`\`\`lang\n[completion_tokens]\n\`\`\`).

Return \`None\` if you think the code is already complete.`
const prompt = `\`\`\`{language}
{before}<completion_tokens>{after}
\`\`\`
`

export async function editorCodeCompletion(
	before: string,
	after: string,
	lang: string,
	abortController: AbortController
) {
	codeCompletionLoading.set(true)
	const messages: ChatCompletionMessageParam[] = [
		{
			role: 'system',
			content: systemPrompt
		},
		{
			role: 'user',
			content: prompt
				.replace('{language}', lang)
				.replace('{before}', before)
				.replace('{after}', after)
		}
	]

	try {
		const result = await getNonStreamingCompletion(messages, abortController, 'gpt-3.5-turbo-1106')

		const match = result.match(/```[a-zA-Z]+\n([\s\S]*?)\n```/)

		let completion = match?.[1] || ''
		if (completion) {
			const previousIndent = before.match(/(\n|^)([ \t]*)$/)?.[2]
			if (previousIndent) {
				completion = completion.replace(/\n/g, `\n${previousIndent}`)
			}
		}

		return completion
	} catch (err) {
		if (err.message !== 'Request was aborted.') {
			console.log(err)
		}
	} finally {
		codeCompletionLoading.set(false)
	}
}
