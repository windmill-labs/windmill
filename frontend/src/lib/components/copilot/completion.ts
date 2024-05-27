import type { ChatCompletionMessageParam } from 'openai/resources/chat/index.mjs'
import { getNonStreamingCompletion } from './lib'
import { codeCompletionLoading } from '$lib/stores'

const systemPrompt = `You are a code completion assistant, return the code that should go instead of the <completion_tokens>.

- Only return the completion tokens. Do not include the surrounding code.
- Wrap the completion tokens in a code block (\`\`\`{language}\n<completion_tokens>\n\`\`\`).
- Maintain correct indentation based on the context. Take into account whether there are whitespaces or tabs before the completion tokens.
- You might need to add additional line breaks at the beginning or end of the completion tokens to make the code syntactically correct.
- Pay attention to not include tokens that are already present in the code, particularly after the completion like parenteses, brackets, etc.
- Return None with no code block if you think the code is already complete.

Examples:

User:
\`\`\`typescript
function greet() {
  <completion_tokens>
}
\`\`\`
Assistant:
\`\`\`typescript
console.log('Hello, world!')
\`\`\`

User:
\`\`\`python
def main(name: str):
    // log the name <completion_tokens>
\`\`\`
Assistant:
\`\`\`python
\n    print(name)
\`\`\`

User:
\`\`\`typescript
function multiplyNumbers(<completion_tokens>)
\`\`\`
Assistant:
\`\`\`typescript
number1: number, number2: number
\`\`\`

User:
\`\`\`python
def greet():
    <completion_tokens>
\`\`\`
Assistant:
\`\`\`python
print("Hello World!")
\`\`\`

User:
\`\`\`typescript
function multiplyNumbers(number1: number, number2: number) {<completion_tokens>}
\`\`\`
Assistant:
\`\`\`typescript
\n  return number1 * number2\n
\`\`\`


`
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
		const result = await getNonStreamingCompletion(messages, abortController)

		const match = result.match(/```[a-zA-Z]+\n([\s\S]*?)\n```/)

		let completion = match?.[1] || ''

		return completion
	} catch (err) {
		if (err.message !== 'Request was aborted.') {
			console.log(err)
		}
	} finally {
		codeCompletionLoading.set(false)
	}
}
