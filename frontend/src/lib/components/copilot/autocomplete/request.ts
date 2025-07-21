import { copilotInfo } from '$lib/stores'
import { get } from 'svelte/store'

import { getFimCompletion } from '../lib'
import { getLangContext } from '../chat/script/core'
import { type ScriptLang } from '$lib/gen/types.gen'
import { getCommentSymbol } from '../utils'
import type { editor } from 'monaco-editor'

function comment(commentSymbol: string, text: string) {
	return text
		.split('\n')
		.map((line) => `${commentSymbol} ${line}`)
		.join('\n')
}

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
	const langContext = getLangContext(context.scriptLang)

	const commentSymbol = getCommentSymbol(context.scriptLang)

	if (langContext) {
		let commentedContext = comment(commentSymbol, langContext)
		if (context.markers.length > 0) {
			const markersLines = comment(commentSymbol, context.markers.map((m) => m.message).join('\n'))
			commentedContext =
				commentedContext + comment(commentSymbol, '\nDIAGNOSTICS:\n') + markersLines
		}
		if (context.libraries) {
			commentedContext =
				commentedContext +
				comment(commentSymbol, '\nLIBRARY AVAILABLE COMPLETIONS:\n') +
				comment(commentSymbol, context.libraries)
		}
		context.prefix = commentedContext + '\n' + context.prefix
	}

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
