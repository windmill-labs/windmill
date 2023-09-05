import { ScriptService, type Script } from '$lib/gen'
import { addResourceTypes, deltaCodeCompletion, getNonStreamingCompletion } from './lib'
import type { Writable } from 'svelte/store'
import type Editor from '../Editor.svelte'
import type { Drawer } from '../common'

export type FlowCopilotModule = {
	id: string
	type: 'trigger' | 'script'
	description: string
	code: string
	source: 'hub' | 'custom' | undefined
	hubCompletions: {
		path: string
		summary: string
		approved: boolean
		kind: string
		app: string
		ask_id: number
	}[]
	selectedCompletion:
		| {
				path: string
				summary: string
				approved: boolean
				kind: string
				app: string
				ask_id: number
		  }
		| undefined
	editor?: Editor
}

export type FlowCopilotContext = {
	drawerStore: Writable<Drawer | undefined>
	modulesStore: Writable<FlowCopilotModule[]>
	currentStepStore: Writable<string | undefined>
}

const systemPrompt = `You write code as instructed by the user. Only output code. Wrap the code in a code block. 
Put explanations directly in the code as comments.

Here's how interactions have to look like:
user: {sample_question}
assistant: \`\`\`typescript
{code}
\`\`\``

const additionalInformation = `Additional information: We have to export a "main" function like this: "export async function main(...)" and specify the parameter types but do not call it.
You have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}
Only use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.`

const triggerPrompt = `I'm building a workflow which is a sequence of script steps. Write the first script in typescript (deno environment) which should check for {description} and return an array.
You can use "const {state_name}: {state_type} = getState(...)" and "setState(...)" from "npm:windmill-client@1" to maintain state across runs.

${additionalInformation}`

const firstActionPrompt = `I'm building a workflow which is a sequence of script steps. Write a script in typescript (deno environment) which should {description}.
Return the script's output.

${additionalInformation}`

const actionPrompt = `I'm building a workflow which is a sequence of script steps. Write a script in typescript (deno environment) which should {description} using as a parameter called "prev_output" the output of the previous script.
Infer the type of "prev_output" from the previous script: \`\`\`deno\n{prevCode}\n\`\`\`.
Return the script's output.

${additionalInformation}`

const loopGluePrompt = `I'm building a workflow which is a sequence of script steps. 
My current step code has the following inputs: {inputs}. 
Determine what to pass as inputs. You can only use the following:
- \`flow_input\` (javascript object): general inputs that are passed to the workflow, you can assume any object properties.
- \`flow_input.iter.value\` (javascript object): it is ONE ELEMENT of the output of the previous step. Infer its type from the previous's step code: \`\`\`deno\n{prevCode}\n\`\`\`

Reply in the following format:
input_name: expr`

const gluePrompt = `I'm building a workflow which is a sequence of script steps. 
My current step code has the following inputs: {inputs}. 
Determine what to pass as inputs. You can only use the following:
- \`flow_input\` (javascript object): general inputs that are passed to the workflow, you can assume any object properties.
- \`prev_output\` (javascript object): previous output is the output of the previous step. Infer its type from the previous's step code: \`\`\`deno\n{prevCode}\n\`\`\`

Reply in the following format:
input_name: expr`

export async function stepCopilot(
	module: FlowCopilotModule,
	deltaCodeStore: Writable<string>,
	prevCode: string,
	abortController: AbortController
) {
	if (module.source === undefined) {
		throw new Error('Module not configured')
	}
	if (module.source === 'hub' && module.selectedCompletion) {
		const hubScript = await ScriptService.getHubScriptByPath({
			path: module.selectedCompletion.path
		})
		deltaCodeStore.set(hubScript.content)
		return hubScript.content
	} else {
		let prompt =
			module.type === 'trigger'
				? triggerPrompt
				: prevCode.length > 0
				? actionPrompt
				: firstActionPrompt
		prompt = prompt.replace('{description}', module.description).replace('{prevCode}', prevCode)
		prompt = await addResourceTypes(
			{
				type: 'gen',
				language: 'deno' as Script.language,
				description: module.description,
				dbSchema: undefined
			},
			prompt
		)
		const code = await deltaCodeCompletion(
			[
				{
					role: 'system',
					content: systemPrompt
				},
				{
					role: 'user',
					content: prompt
				}
			],
			deltaCodeStore,
			abortController
		)
		return code
	}
}

export async function glueCopilot(
	inputs: string[],
	prevCode: string,
	isLoop: boolean,
	abortController: AbortController
) {
	let response = await getNonStreamingCompletion(
		[
			{
				role: 'user',
				content: (isLoop ? loopGluePrompt : gluePrompt)
					.replace('{inputs}', inputs.join(', '))
					.replace('{prevCode}', prevCode)
			}
		],
		abortController
	)

	const matches = response.matchAll(/(.+?): (.+)/g)

	const result: Record<string, string> = {}
	for (const match of matches) {
		const inputName = match[1]
		const inputExpr = match[2].replace(',', '')
		result[inputName] = inputExpr
	}

	return result
}
