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
		'You are a code completion assistant. You are given three important contexts (<LANGUAGE CONTEXT>, <DIAGNOSTICS>, <LIBRARY METHODS>) to help you complete the code. IMPORTANT: Make sure to use the correct signature from the <LIBRARY METHODS> when given.\n'
	contextLines += '<LANGUAGE CONTEXT>\n'
	contextLines += getLangContext(context.scriptLang) + '\n'
	contextLines += '</LANGUAGE CONTEXT>\n'
	if (context.markers.length > 0) {
		contextLines += '<DIAGNOSTICS>\n'
		contextLines += context.markers.map((m) => m.message).join('\n') + '\n'
		contextLines += '</DIAGNOSTICS>\n'
	}
	if (context.libraries.length > 0) {
		contextLines += '<LIBRARY METHODS>\n'
		contextLines += context.libraries + '\n'
		contextLines += '</LIBRARY METHODS>\n'
	}

	context.prefix = contextLines + '\n' + context.prefix
	console.log('context.prefix', context.prefix)

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
