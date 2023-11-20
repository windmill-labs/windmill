import type { ChatCompletionMessageParam } from 'openai/resources/chat'
import { getNonStreamingCompletion } from './lib'
import { codeCompletionLoading } from '$lib/stores'

const systemPrompt =
	'You are a code completion assistant, return the code that should go inside the <completion></completion> tags. If you add a line break, take into account the indentation of the code. You can also not return anything if you think the code is already complete.'
const prompt = `complete the following code:
\`\`\`{language}
{before}<completion></completion>{after}
\`\`\``

const exampleUser = prompt
	.replace('{language}', 'python')
	.replace(
		'{before}',
		`def main(number):
    # print the number and divide it by 4
    `
	)
	.replace('{after}', `\n\n    return number`)
const exampleAssistant = `<completion>print(number)
		number = number / 4</completion>`

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
			content: exampleUser
		},
		{
			role: 'assistant',
			content: exampleAssistant
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
		const result = await getNonStreamingCompletion(messages, abortController, 'gpt-3.5-turbo')
		const completion = result.match(/<completion>(.*)<\/completion>/s)?.[1] || ''

		return completion
	} catch (err) {
		if (err.message !== 'Request was aborted.') {
			console.log(err)
		}
	} finally {
		codeCompletionLoading.set(false)
	}
}
