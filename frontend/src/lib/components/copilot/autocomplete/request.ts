import { copilotInfo } from '$lib/stores'
import { get } from 'svelte/store'
import { getFimCompletion } from '../lib'
import { getLangContext } from '../chat/script/core'
import { type ScriptLang } from '$lib/gen/types.gen'
import type { editor } from 'monaco-editor'

export async function autocompleteRequest(
	context: {
		prefix: string
		suffix: string
		scriptLang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json'
		markers: editor.IMarker[]
		libraries: string
	},
	abortController: AbortController
) {
	let contextLines =
		'You are a code completion assistant. You are given three important contexts (<LANGUAGE CONTEXT>, <DIAGNOSTICS>, <LIBRARY METHODS>) to help you complete the code.\n'
	contextLines += '<LANGUAGE CONTEXT>\n'
	contextLines += getLangContext(context.scriptLang) + '\n'
	contextLines += '</LANGUAGE CONTEXT>\n'
	contextLines += '<DIAGNOSTICS>\n'
	contextLines += context.markers.map((m) => m.message).join('\n') + '\n'
	contextLines += '</DIAGNOSTICS>\n'
	contextLines += '<LIBRARY METHODS>\n'
	contextLines += context.libraries + '\n'
	contextLines += '</LIBRARY METHODS>\n'

	context.prefix = contextLines + '\n' + context.prefix

	const providerModel = get(copilotInfo).codeCompletionModel

	if (!providerModel) {
		throw new Error('No code completion model selected')
	}

	try {
		const completion = await getFimCompletion(
			context.prefix,
			context.suffix,
			providerModel,
			abortController
		)

		return completion
	} catch (err) {
		if (!abortController.signal.aborted) {
			console.log('Could not generate autocomplete', err.message)
		}
	}
}
