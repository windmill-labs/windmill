import { copilotInfo } from '$lib/stores'
import { get } from 'svelte/store'

import { getFimCompletion } from '../lib'
import { getLangContext } from '../chat/script/core'
import { type ScriptLang } from '$lib/gen/types.gen'
import { getCommentSymbol } from '../utils'
import type { editor } from 'monaco-editor'

export async function autocompleteRequest(
	context: {
		prefix: string
		suffix: string
		scriptLang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json'
		markers: editor.IMarker[]
	},
	abortController: AbortController
) {
	const langContext = getLangContext(context.scriptLang)

	const commentSymbol = getCommentSymbol(context.scriptLang)

	if (langContext) {
		const contextLines = langContext.split('\n')
		const markersLines = context.markers.map((m) => m.message)
		let commentedContext = contextLines.map((line) => `${commentSymbol} ${line}`).join('\n')
		if (markersLines.length > 0) {
			commentedContext = commentedContext + '\nDIAGNOSTICS:\n' + markersLines.join('\n')
		}
		context.prefix = commentedContext + '\n' + context.prefix
	}

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

		console.log('completion', completion)

		return completion
	} catch (err) {
		if (!abortController.signal.aborted) {
			console.log('Could not generate autocomplete', err.message)
		}
	}
}
