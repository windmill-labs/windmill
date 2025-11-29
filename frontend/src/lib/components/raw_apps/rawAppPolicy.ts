import type { Policy, ScriptLang } from '$lib/gen'
import { collectStaticFields, hash, type TriggerableV2 } from '../apps/editor/commonAppUtils'
import { isRunnableByName, isRunnableByPath, type InlineScript, type RunnableWithFields } from '../apps/inputType'

export async function updateRawAppPolicy(
	runnables: Record<string, Runnable>,
	currentPolicy: Policy | undefined
): Promise<Policy> {
	const triggerables_v2 = Object.fromEntries(
		(await Promise.all(
			Object.entries(runnables).map(async ([id, runnable]) => {
				return await processRunnable(id, runnable, runnable?.fields ?? {})
			})
		)) as [string, TriggerableV2][]
	)
	return {
		...currentPolicy,
		triggerables_v2
	}
}

type RunnableWithInlineScript = RunnableWithFields & {
	inlineScript?: InlineScript & { language: ScriptLang }
}
export type Runnable = RunnableWithInlineScript | undefined

async function processRunnable(
	id: string,
	runnable: Runnable,
	fields: Record<string, any>
): Promise<[string, TriggerableV2] | undefined> {
	const staticInputs = collectStaticFields(fields)
	const allowUserResources: string[] = Object.entries(fields)
		.map(([k, v]) => {
			return v['allowUserResources'] ? k : undefined
		})
		.filter(Boolean) as string[]

	if (isRunnableByName(runnable)) {
		let hex = await hash(runnable.inlineScript?.content)
		console.log('hex', hex, id)
		return [
			`${id}:rawscript/${hex}`,
			{
				static_inputs: staticInputs,
				one_of_inputs: {},
				allow_user_resources: allowUserResources
			}
		]
	} else if (isRunnableByPath(runnable)) {
		let prefix = runnable.runType !== 'hubscript' ? runnable.runType : 'script'
		return [
			`${id}:${prefix}/${runnable.path}`,
			{
				static_inputs: staticInputs,
				one_of_inputs: {},
				allow_user_resources: allowUserResources
			}
		]
	}
}
