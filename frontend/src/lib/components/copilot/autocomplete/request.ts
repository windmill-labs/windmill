import { codeCompletionLoading, copilotInfo } from '$lib/stores'
import { get } from 'svelte/store'

import { getNonStreamingCompletion } from '../lib'
import { getLangContext } from '../chat/script/core'
import { type ScriptLang } from '$lib/gen/types.gen'

const AUTOCOMPLETE_SYSTEM_PROMPT = `You're a code assistant. Your task is to help the user write code by suggesting the next edit for the user.

As an intelligent code assistant, your role is to analyze what the user has been doing and then to suggest the most likely next modification.

## Task

Your task is to rewrite the <EDITABLE_CODE> section of the code I send you to include an edit the user should make.
The <CURSOR> tag marks the position of the user's cursor.

Follow the following criteria.

### High-level Guidelines

- Consider the overall intent and direction of the changes
- Take into account what the user has been doing
- Maintain the code style and formatting conventions of the language used in the file
- Your edit suggestions **must** be small and self-contained. Example: if there are two statements that logically need to be added together, suggest them together instead of one by one.

### Constraints

- Preserve indentation and braces/parentheses/brackets balance.
- Prefer suggesting actual implementations over suggesting placeholders
- Dont explain the code, only return the complete <EDITABLE_CODE> section with your edits. DO NOT return any code after the <EDITABLE_CODE> tag.
- If there are no useful edits to make, return the the <EDITABLE_CODE> section unmodified, without the <CURSOR> tag.
- Never include the <CURSOR> tag in the response.
- Never remove line breaks inside the <EDITABLE_CODE> section.`

const AUTOCOMPLETE_USER_PROMPT = `
WINDMILL LANGUAGE CONTEXT:
{lang_context}

<CODE>
{prefix}<EDITABLE_CODE>
{modifiablePrefix}<CURSOR>{modifiableSuffix}
</EDITABLE_CODE>
{suffix}</CODE>

Return the EDITABLE_CODE section in the form \`\`\`{language}
<EDITABLE_CODE>
...complete editable code section with your modifications
</EDITABLE_CODE>
\`\`\``

function postProcessing(response: string) {
	const code = response.match(/<EDITABLE_CODE>\n?(.*?)\n?<\/EDITABLE_CODE>/s)?.[1]

	if (!code) {
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
		scriptLang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json'
		events: string[]
	},
	abortController: AbortController
) {
	codeCompletionLoading.set(true)
	const systemPrompt = AUTOCOMPLETE_SYSTEM_PROMPT
	const userPrompt = AUTOCOMPLETE_USER_PROMPT.replace(
		'{lang_context}',
		getLangContext(context.scriptLang)
	)
		.replace('{prefix}', context.prefix)
		.replace('{modifiablePrefix}', context.modifiablePrefix)
		.replace('{modifiableSuffix}', context.modifiableSuffix)
		.replace('{suffix}', context.suffix)
		.replace('{language}', context.language)
		.replace('{events}', context.events.join('\n\n'))

	const info = get(copilotInfo)

	const providerModel = info.codeCompletionModel

	if (!providerModel) {
		throw new Error('No code completion model selected')
	}

	try {
		const completion = await getNonStreamingCompletion(
			[
				{ role: 'system', content: systemPrompt },
				{ role: 'user', content: userPrompt }
			],
			abortController,
			{
				forceModelProvider: providerModel
			}
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
