import type { Script, FlowModule } from '$lib/gen'
import { addResourceTypes, deltaCodeCompletion, getNonStreamingCompletion } from './lib'
import type { Writable } from 'svelte/store'
import type Editor from '../Editor.svelte'
import type { Drawer } from '../common'
import { scriptLangToEditorLang } from '$lib/scripts'

export type FlowCopilotModule = {
	id: string
	type: 'trigger' | 'script'
	description: string
	code: string
	source: 'hub' | 'custom' | undefined
	lang: 'bun' | 'python3' | undefined
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
	genFlow: ((i: number, modules: FlowModule[], stepOnly?: boolean) => Promise<void>) | undefined
}

const systemPrompt = `You write code as instructed by the user. Only output code. Wrap the code in a code block. 
Put explanations directly in the code as comments.

Here's how interactions have to look like:
user: {sample_question}
assistant: \`\`\`{codeLang}
{code}
\`\`\``

const additionalInfos: {
	bun: string
	python3: string
} = {
	bun: `<contextual_information>
We have to export a "main" function like this: "export async function main(...)" and specify the parameter types but do not call it.
If needed, the standard fetch method is available globally, do not import it.
You can take as parameters resources which are dictionaries containing credentials or configuration information. 
The resource type name has to be exactly as specified.
<resourceTypes>
{resourceTypes}
</resourceTypes>
Only define the type for resources that are actually needed to achieve the function purpose. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.
</contextual_information>`,
	python3: `<contextual_information>
We have to export a "main" function and specify the parameter types but do not call it.
You can take as parameters resources which are dictionaries containing credentials or configuration information. 
The resource type name has to be exactly as specified (has to be IN LOWERCASE).
<resourceTypes>
{resourceTypes}
</resourceTypes>
Only define the type for resources that are actually needed to achieve the function purpose. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.
</contextual_information>`
}

const triggerPrompts: {
	bun: string
	python3: string
} = {
	bun: `I'm building a workflow which is a sequence of script steps. Write the first script in {codeLang} which should check for {description} and return an array.
You can use "const {state_name}: {state_type} = getState(...)" and "setState(...)" from "npm:windmill-client@1" to maintain state across runs.

{additionalInformation}`,
	python3: `I'm building a workflow which is a sequence of script steps. Write the first script in {codeLang} which should check for {description} and return an array.
You can use get_state and set_state from wmill to maintain state across runs.

{additionalInformation}`
}

const firstActionPrompt = `I'm building a workflow which is a sequence of script steps. Write a script in {codeLang} which should {description}.
Return the script's output.

{additionalInformation}`

const inferTypePrompt =
	'Infer the type of "prev_output" from the previous\'s step code: ```{codeLang}\n{prevCode}\n```'

const actionPrompt = `I'm building a workflow which is a sequence of script steps. Write a script in {codeLang} which should {description}. It should take a parameter called "prev_output" which contains the output of the previous script.
{inferTypePrompt}
Return the script's output.

{additionalInformation}`

const inferTypeLoopPrompt =
	'Infer the type of "prev_output" from the previous\'s step code: ```{codeLang}\n{prevCode}\n```, keeping in mind that it is ONE ELEMENT of the output of the previous step.'

const loopActionPrompt = `I'm building a workflow which is a sequence of script steps. Write a script in {codeLang} which should {description}. It should take a parameter called "prev_output" which contains ONE ELEMEMT of the output of the previous script.
{inferTypePrompt}
Return the script's output.

{additionalInformation}`

const inferTypeGluePrompt =
	"Infer its type from the previous's step code: ```{codeLang}\n{prevCode}\n```"

const loopGluePrompt = `I'm building a workflow which is a sequence of script steps. 
My current step code has the following inputs: {inputs}. 
Determine what to pass as inputs. You can only use the following:
- \`flow_input\` (javascript object): general inputs that are passed to the workflow, you can assume any object properties.
- \`flow_input.iter.value\` (javascript object): it is ONE ELEMENT of the output of the previous step. {inferTypeGluePrompt}

Reply in the following format:
input_name: expr`

const gluePrompt = `I'm building a workflow which is a sequence of script steps. 
My current step code has the following inputs: {inputs}. 
Determine what to pass as inputs. You can only use the following:
- \`flow_input\` (javascript object): general inputs that are passed to the workflow, you can assume any object properties.
- \`prev_output\` (javascript object): previous output is the output of the previous step. {inferTypeGluePrompt}

Reply in the following format:
input_name: expr`

export async function stepCopilot(
	module: FlowCopilotModule,
	deltaCodeStore: Writable<string>,
	prevCode: string,
	prevLang: Script.language | undefined,
	isFirstAction: boolean,
	isFirstInLoop: boolean,
	abortController: AbortController
) {
	if (module.source !== 'custom') {
		throw new Error('Not a custom module')
	}
	const lang = module.lang ?? 'bun'
	const codeLang = lang === 'python3' ? 'python' : 'typescript (Node.js)'
	let prompt =
		module.type === 'trigger'
			? triggerPrompts[lang]
			: isFirstAction
			? firstActionPrompt
			: isFirstInLoop
			? loopActionPrompt
			: actionPrompt
	prompt = prompt
		.replace('{codeLang}', codeLang)
		.replace(
			'{inferTypePrompt}',
			prevCode.length > 0 && prevLang
				? (isFirstInLoop ? inferTypeLoopPrompt : inferTypePrompt)
						.replace('{prevCode}', prevCode)
						.replace('{codeLang}', scriptLangToEditorLang(prevLang))
				: ''
		)
		.replace('{additionalInformation}', additionalInfos[lang])
		.replace('{description}', module.description)
	prompt = await addResourceTypes(
		{
			type: 'gen',
			language: lang as Script.language,
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

export async function glueCopilot(
	inputs: string[],
	prevCode: string,
	prevLang: Script.language | undefined,
	isFirstInLoop: boolean,
	abortController: AbortController
) {
	let response = await getNonStreamingCompletion(
		[
			{
				role: 'user',
				content: (isFirstInLoop ? loopGluePrompt : gluePrompt)
					.replace('{inputs}', inputs.join(', '))
					.replace(
						'{inferTypeGluePrompt}',
						prevCode.length > 0 && prevLang
							? inferTypeGluePrompt
									.replace('{prevCode}', prevCode)
									.replace('{codeLang}', scriptLangToEditorLang(prevLang))
							: ''
					)
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
