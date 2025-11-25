import type { Policy, ScriptLang } from '$lib/gen'
import { collectStaticFields, hash, type TriggerableV2 } from '../apps/editor/commonAppUtils'
import type { InlineScript, RunnableWithFields } from '../apps/inputType'

export async function updateRawAppPolicy(
	runnables: Record<string, Runnable>,
	currentPolicy: Policy | undefined
): Promise<Policy> {
	const triggerables_v2 = Object.fromEntries(
		(await Promise.all(
			Object.values(runnables).map(async (runnable) => {
				return await processRunnable(runnable?.name ?? '', runnable, runnable?.fields ?? {})
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

	if (runnable?.type == 'runnableByName') {
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
	} else if (runnable?.type == 'runnableByPath') {
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
