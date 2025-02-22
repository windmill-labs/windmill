import type { AIProvider } from '$lib/gen'
import { codeCompletionLoading } from '$lib/stores'

import { getNonStreamingCompletion } from '../lib'

const AUTOCOMPLETE_SYSTEM_PROMPT = `
You are a coding assistant that predicts the user's next modifications to the code. Your task is to anticipate the next changes the user will make based on the provided context and return the modified code.

The user will provide the full code inside the <CODE> tag and the code that is modifiable inside the <EDITABLE_CODE> tag. The user cursor position is marked by the <CURSOR> tag.

### Instructions:
1. Return the complete <EDITABLE_CODE> section with your modifications inside it.
2. DO NOT return any code after the <EDITABLE_CODE> tag.
2. If you delete parentheses, brackets, and braces, you have to balance the code afterwards.
3. Make sure to remove the <CURSOR> tag from your output.


Follow these instructions carefully to generate accurate predictions.
`

const AUTOCOMPLETE_USER_PROMPT = `
<CODE>
{prefix}<EDITABLE_CODE>
{modifiablePrefix}<CURSOR>{modifiableSuffix}
</EDITABLE_CODE>
{suffix}</CODE>

Return the EDITABLE_CODE section in the form \`\`\`{language}
<EDITABLE_CODE>
...complete editable code with your modifications while making sure to balance parentheses, brackets, and braces.
</EDITABLE_CODE>
\`\`\`).`

function postProcessing(response: string) {
	const code = response.match(/<EDITABLE_CODE>\n?(.*?)\n?<\/EDITABLE_CODE>/s)?.[1]

	if (!code) {
		console.log('raw response', response)
		throw new Error('No code found in response')
	}
	return code
}

export async function autocompleteRequest(
	context: {
		prefix: string
		modifiablePrefix: string
		modifiableSuffix: string
		suffix: string
		language: string
	},
	abortController: AbortController,
	aiProvider: AIProvider
) {
	codeCompletionLoading.set(true)
	const systemPrompt = AUTOCOMPLETE_SYSTEM_PROMPT
	const userPrompt = AUTOCOMPLETE_USER_PROMPT.replace('{prefix}', context.prefix)
		.replace('{modifiablePrefix}', context.modifiablePrefix)
		.replace('{modifiableSuffix}', context.modifiableSuffix)
		.replace('{suffix}', context.suffix)
		.replace('{language}', context.language)

	console.log('userprompt', userPrompt)

	try {
		const completion = await getNonStreamingCompletion(
			[
				{ role: 'system', content: systemPrompt },
				{ role: 'user', content: userPrompt }
			],
			abortController,
			aiProvider,
			undefined,
			undefined,
			'codestral-latest'
		)

		console.log('completion', completion)

		return postProcessing(completion)
	} catch (err) {
		if (err.message !== 'Request was aborted.') {
			console.log(err)
		}
	} finally {
		codeCompletionLoading.set(false)
	}
}
