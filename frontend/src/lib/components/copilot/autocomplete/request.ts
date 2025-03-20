import { codeCompletionLoading } from '$lib/stores'

import { getNonStreamingCompletion } from '../lib'

const AUTOCOMPLETE_SYSTEM_PROMPT = `You're a code assistant. Your task is to help the user write code by suggesting the next edit for the user.

As an intelligent code assistant, your role is to analyze what the user has been doing and then to suggest the most likely next modification.
The user last changes are marked by the <EVENTS> tag. Last event is the most recent one.

## Task

Your task is to rewrite the <EDITABLE_CODE> section of the code I send you to include an edit the user should make.
The <CURSOR> tag marks the position of the user's cursor.

Follow the following criteria.

### High-level Guidelines

- Predict logical next changes based on the edit patterns you've observed
- Consider the overall intent and direction of the changes
- Take into account what the user has been doing

### Constraints

- Your edit suggestions **must** be small and self-contained. Example: if there are two statements that logically need to be added together, suggest them together instead of one by one.
- Preserve indentation and braces/parentheses/brackets balance.
- Do not suggest re-adding code the user has recently deleted
- Do not suggest deleting lines that the user has recently inserted
- Prefer completing what the user just typed over suggesting to delete what they typed

### Best Practices

- Fix any syntax errors or inconsistencies in the code
- Maintain the code style and formatting conventions of the language used in the file
- Add missing syntactic elements, such as closing parentheses or semicolons
- Remove the complete <EDITABLE_CODE> section with your edits.

- If there are no useful edits to make, return the code unmodified.
- Don't explain the code, just rewrite it to include the next, most probable change.
- Never include the <CURSOR> tag in the response.`

// const AUTOCOMPLETE_SYSTEM_PROMPT = `
// You are a coding assistant that predicts the user's next modifications to the code. Your task is to anticipate the next changes the user will make based on the provided context and return the modified code.

// The user will provide the full code inside the <CODE> tag and the code that is modifiable inside the <EDITABLE_CODE> tag. The user cursor position is marked by the <CURSOR> tag.

// ### Instructions:
// 1. Return the complete <EDITABLE_CODE> section with your modifications inside it.
// 2. DO NOT return any code after the <EDITABLE_CODE> tag.
// 2. If you delete parentheses, brackets, and braces, you have to balance the code afterwards.
// 3. Make sure to remove the <CURSOR> tag from your output.

// Follow these instructions carefully to generate accurate predictions.
// `

const AUTOCOMPLETE_USER_PROMPT = `
<CODE>
{prefix}<EDITABLE_CODE>
{modifiablePrefix}<CURSOR>{modifiableSuffix}
</EDITABLE_CODE>
{suffix}</CODE>

Return the EDITABLE_CODE section in the form \`\`\`{language}
<EDITABLE_CODE>
...complete editable code section with your modifications, including balancing braces/parentheses/brackets/indentation
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
		events: string[]
	},
	abortController: AbortController
) {
	codeCompletionLoading.set(true)
	const systemPrompt = AUTOCOMPLETE_SYSTEM_PROMPT
	const userPrompt = AUTOCOMPLETE_USER_PROMPT.replace('{prefix}', context.prefix)
		.replace('{modifiablePrefix}', context.modifiablePrefix)
		.replace('{modifiableSuffix}', context.modifiableSuffix)
		.replace('{suffix}', context.suffix)
		.replace('{language}', context.language)
		.replace('{events}', context.events.join('\n\n'))
	console.log('events', context.events)

	try {
		const completion = await getNonStreamingCompletion(
			[
				{ role: 'system', content: systemPrompt },
				{ role: 'user', content: userPrompt }
			],
			abortController
		)

		return postProcessing(completion)
	} catch (err) {
		if (!abortController.signal.aborted) {
			console.log('Could not generate autocomplete', err.message)
		}
	} finally {
		codeCompletionLoading.set(false)
	}
}
