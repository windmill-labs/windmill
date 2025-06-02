import {
	ScriptService,
	type FlowModule,
	type HubScriptKind,
	type InputTransform,
	type PathScript,
	type RawScript,
	type Script
} from '$lib/gen'
import { scriptLangToEditorLang } from '$lib/scripts'
import type { Writable } from 'svelte/store'
import type Editor from '../Editor.svelte'
import type { Drawer } from '../common'
import { addResourceTypes, deltaCodeCompletion, getNonStreamingCompletion } from './lib'

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
		kind: HubScriptKind
		app: string
		ask_id: number
		id: number
		version_id: number
	}[]
	selectedCompletion:
		| {
				path: string
				summary: string
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
	genFlow: ((i: number, stepOnly?: boolean) => Promise<void>) | undefined
	shouldUpdatePropertyType: Writable<{
		[key: string]: 'static' | 'javascript' | undefined
	}>
	stepInputsLoading: Writable<boolean>
	generatedExprs: Writable<{
		[key: string]: string | undefined
	}>
	exprsToSet: Writable<{
		[key: string]: InputTransform | undefined
	}>
}

const systemPrompt = `You are a helpful coding assistant for Windmill, a developer platform for running scripts. You write code as instructed by the user. Each user message includes some contextual information which should guide your answer.
Only output code. Wrap the code in a code block.
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
You have to write TypeScript code and export a "main" function like this: "export async function main(...)" and specify the parameter types but do not call it. You should generally return the result.
The fetch standard method is available globally.
You can take as parameters resources which are dictionaries containing credentials or configuration information. For Windmill to correctly detect the resources to be passed, the resource type name has to be exactly as specified in the following list:
<resourceTypes>
{resourceTypes}
</resourceTypes>
You need to define the type of the resources that are needed before the main function, but only include them if they are actually needed to achieve the function purpose.
The resource type name has to be exactly as specified (no resource suffix). If the type name conflicts with any imported methods, you have to rename the imported method with the conflicting name.
</contextual_information>`,
	python3: `<contextual_information>
You have to write a function in Python called "main". Specify the parameter types. Do not call the main function. You should generally return the result.
You can take as parameters resources which are dictionaries containing credentials or configuration information. For Windmill to correctly detect the resources to be passed, the resource type name has to be exactly as specified in the following list:
<resourceTypes>
{resourceTypes}
</resourceTypes>
You need to define the type of the resources that are needed before the main function, but only include them if they are actually needed to achieve the function purpose.
The resource type name has to be exactly as specified (has to be IN LOWERCASE). If the type name conflicts with any imported methods, you have to rename the imported method with the conflicting name.
<contextual_information>`
}

const triggerPrompts: {
	bun: string
	python3: string
} = {
	bun: `I'm building a workflow which is a sequence of script steps. Write the first script in {codeLang} which should check for {description} and return an array.
To maintain state across runs, you can use "const {state_name}: {state_type} = await getState()" and "await setState(value: any)" which you have to import like this: import { getState, setState } from "windmill-client@1"

{additionalInformation}`,
	python3: `I'm building a workflow which is a sequence of script steps. Write the first script in {codeLang} which should check for {description} and return an array.
To maintain state across runs, you can use get_state() and set_state(value) which you have to import like this: from wmill import get_state, set_state

{additionalInformation}`
}

// const preprocessorPrompts: {
// 	bun: string
// 	python3: string
// } = {
// 	bun: `I'm building a workflow which is a sequence of script steps. Write the preprocessor step in {codeLang} which should check for {description} and return an array.
// The preprocessor step is executed before flow begins to map trigger specific inputs to the flow inputs.

// Here is an example of what the preprocessor step should look like:
// \`\`\`{codeLang}
// export async function preprocessor(
// 	wm_trigger: {
// 		kind: 'http' | 'email' | 'webhook',
// 		http?: {
// 			route: string // The route path, e.g. "/users/:id"
// 			path: string // The actual path called, e.g. "/users/123"
// 			method: string
// 			params: Record<string, string>
// 			query: Record<string, string>
// 			headers: Record<string, string>
// 		}
// 	},
// 	/* your other args */
// ) {
// 	return {
// 		// return the args to be passed to the flow
// 	}
// }
// \`\`\`

// {additionalInformation}`,
// 	python3: `I'm building a workflow which is a sequence of script steps. Write the preprocessor step in {codeLang} which should check for {description} and return an array.
// The preprocessor step is executed before flow begins to map trigger specific inputs to the flow inputs.

// Here is an example of what the preprocessor step should look like:
// \`\`\`{codeLang}
// from typing import TypedDict, Literal

// class Http(TypedDict):
// 	route: str # The route path, e.g. "/users/:id"
// 	path: str # The actual path called, e.g. "/users/123"
// 	method: str
// 	params: dict[str, str]
// 	query: dict[str, str]
// 	headers: dict[str, str]

// class WmTrigger(TypedDict):
//     kind: Literal["http", "email", "webhook"]
//     http: Http | None

// def preprocessor(
// 	wm_trigger: WmTrigger,
// 	# your other args
// ):
// 	return {
// 		# return the args to be passed to the flow
// 	}
// \`\`\`

// {additionalInformation}`
// }

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
	"Infer its properties from the previous's step code: ```{codeLang}\n{prevCode}\n```"

const loopGluePrompt = `I'm building a workflow which is a sequence of script steps. 
My current step code has the following inputs: {inputs}. 
Determine for each input, what to pass from the following:
- \`flow_input\` (javascript object): general inputs that are passed to the workflow, you can assume any object properties (snake case).
- \`flow_input.iter.value\` (javascript object): it is ONE ELEMENT of the output of the previous step. {inferTypeGluePrompt}

Reply with the most probable answer, do not explain or discuss.
Your answer has to be in the following format (one line per input):
input_name: expr`

const gluePrompt = `I'm building a workflow which is a sequence of script steps. 
My current step code has the following inputs: {inputs}. 
Determine for each input, what to pass from the following:
- \`flow_input\` (javascript object): general inputs that are passed to the workflow, you can assume any object properties (snake case).
- \`results.{prevId}\` (javascript object): output of the previous step. {inferTypeGluePrompt}

Reply with the most probable answer, do not explain or discuss.
Your answer has to be in the following format (one line per input):
input_name: expr`

async function getPreviousStepContent(
	pastModule: FlowModule & {
		value: RawScript | PathScript
	},
	workspace: string
) {
	if (pastModule.value.type === 'rawscript') {
		return { prevCode: pastModule.value.content, prevLang: pastModule.value.language }
	} else {
		if (pastModule.value.path.startsWith('hub/')) {
			const script = await ScriptService.getHubScriptByPath({
				path: pastModule.value.path
			})
			return { prevCode: script.content, prevLang: script.language as Script['language'] }
		} else if (pastModule.value.hash) {
			const script = await ScriptService.getScriptByHash({
				workspace,
				hash: pastModule.value.hash
			})
			return { prevCode: script.content, prevLang: script.language }
		} else {
			const script = await ScriptService.getScriptByPath({
				workspace,
				path: pastModule.value.path
			})
			return { prevCode: script.content, prevLang: script.language }
		}
	}
}

export async function stepCopilot(
	module: FlowCopilotModule,
	deltaCodeStore: Writable<string>,
	workspace: string,
	pastModule:
		| (FlowModule & {
				value: RawScript | PathScript
		  })
		| undefined,
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
			: // : module.type === 'preprocessor'
				// 	? preprocessorPrompts[lang]
				pastModule === undefined
				? firstActionPrompt
				: isFirstInLoop
					? loopActionPrompt
					: actionPrompt

	const { prevCode, prevLang } = pastModule
		? await getPreviousStepContent(pastModule, workspace)
		: { prevCode: undefined, prevLang: undefined }
	prompt = prompt
		.replace('{codeLang}', codeLang)
		.replace(
			'{inferTypePrompt}',
			prevCode && prevLang
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
			language: lang as Script['language'],
			description: module.description,
			dbSchema: undefined,
			workspace
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
	inputs: Record<string, InputTransform>,
	workspace: string,
	pastModule: FlowModule & {
		value: RawScript | PathScript
	},
	isFirstInLoop: boolean,
	abortController: AbortController
) {
	const { prevCode, prevLang } = await getPreviousStepContent(pastModule, workspace)

	const stringInputs: string[] = []
	for (const inputName in inputs) {
		const input = inputs[inputName]
		if (
			input.type === 'static' &&
			input.value &&
			typeof input.value === 'object' &&
			!Array.isArray(input.value)
		) {
			// nested object
			stringInputs.push(`${inputName} (${Object.keys(input.value).join(', ')})`)
		} else {
			stringInputs.push(inputName)
		}
	}

	let response = await getNonStreamingCompletion(
		[
			{
				role: 'user',
				content: (isFirstInLoop ? loopGluePrompt : gluePrompt)
					.replace('{inputs}', stringInputs.join(', '))
					.replace('{prevId}', pastModule.id)
					.replace(
						'{inferTypeGluePrompt}',
						inferTypeGluePrompt
							.replace('{prevCode}', prevCode)
							.replace('{codeLang}', scriptLangToEditorLang(prevLang))
					)
			}
		],
		abortController
	)

	const matches = response.matchAll(/([a-zA-Z_0-9.]+): (.+)/g)

	const result: Record<string, string> = {}
	const allExprs: Record<string, string> = {}
	for (const match of matches) {
		const inputName = match[1]
		const inputExpr = match[2].replace(',', '')

		allExprs[inputName] = inputExpr
		if (inputName.includes('.')) {
			// nested key returned by copilot (e.g. body.content: ...)
			const [firstKey, ...rest] = inputName.split('.')
			const restStr = rest.join('.')
			if (!result[firstKey]) {
				result[firstKey] = `{\n  "${restStr}": ${inputExpr}\n}`
			} else {
				result[firstKey] = result[firstKey].replace('\n}', `,\n  "${restStr}": ${inputExpr}\n}`)
			}
		} else {
			result[inputName] = inputExpr
		}
	}

	return {
		inputs: result,
		allExprs
	}
}
