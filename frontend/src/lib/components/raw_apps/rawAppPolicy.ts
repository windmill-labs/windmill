import type { Policy, ScriptLang } from '$lib/gen'
import { collectStaticFields, hash, type TriggerableV2 } from '../apps/editor/commonAppUtils'
import {
	isRunnableByName,
	isRunnableByPath,
	type InlineScript,
	type RunnableWithFields
} from '../apps/inputType'

export async function updateRawAppPolicy(
	runnables: Record<string, Runnable>,
	currentPolicy: Policy | undefined
): Promise<Policy> {
	const entries = (
		await Promise.all(
			Object.entries(runnables).map(async ([id, runnable]) => {
				return await processRunnable(id, runnable, runnable?.fields ?? {})
			})
		)
	).filter((entry): entry is [string, TriggerableV2] => entry != null)
	const triggerables_v2 = Object.fromEntries(entries)
	return {
		...currentPolicy,
		triggerables_v2
	}
}

type RunnableWithInlineScript = RunnableWithFields & {
	inlineScript?: InlineScript & { language: ScriptLang }
	delete_after_secs?: number
}
export type Runnable = RunnableWithInlineScript | undefined

function extraFields(
	runnable: RunnableWithInlineScript,
	fields: Record<string, any>
): Partial<Pick<TriggerableV2, 'delete_after_secs' | 'sensitive_inputs'>> {
	const out: Partial<Pick<TriggerableV2, 'delete_after_secs' | 'sensitive_inputs'>> = {}
	if (typeof runnable.delete_after_secs === 'number' && runnable.delete_after_secs >= 0)
		out.delete_after_secs = runnable.delete_after_secs
	const sensitive_inputs = Object.entries(fields)
		.map(([k, v]) => (v['sensitive'] ? k : undefined))
		.filter(Boolean) as string[]
	if (sensitive_inputs.length > 0) out.sensitive_inputs = sensitive_inputs
	return out
}

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
				allow_user_resources: allowUserResources,
				...extraFields(runnable, fields)
			}
		]
	} else if (isRunnableByPath(runnable)) {
		let prefix = runnable.runType !== 'hubscript' ? runnable.runType : 'script'
		return [
			`${id}:${prefix}/${runnable.path}`,
			{
				static_inputs: staticInputs,
				one_of_inputs: {},
				allow_user_resources: allowUserResources,
				...extraFields(runnable, fields)
			}
		]
	}
}
