import type { AIProviderModel, ScriptLang } from '$lib/gen'
import { sleep } from '$lib/utils'
import { editor as meditor, Position, languages, type IDisposable } from 'monaco-editor'
import { LRUCache } from 'lru-cache'
import { autocompleteRequest } from './request'
import { FIM_MAX_TOKENS, getModelContextWindow } from '../lib'
import { setGlobalCSS } from '../shared'
import { get } from 'svelte/store'
import { copilotInfo } from '$lib/stores'

// max ratio of type script completions to context window
const TYPE_SCRIPT_COMPLETIONS_MAX_RATIO = 0.1

// hard limit to max number of type script completions to fetch details for, to avoid performance overhead
const TYPE_SCRIPT_COMPLETIONS_DETAILS_MAX = 100

type CacheCompletion = {
	linePrefix: string
	completion: string
	column: number
}

function filterCompletion(
	completion: string,
	suffix: string,
	shouldReturnMultiline: boolean
): string | undefined {
	const trimmedCompletion = completion.replaceAll('\n', '')
	const trimmedSuffix = suffix.slice(0, FIM_MAX_TOKENS).replaceAll('\n', '')

	if (trimmedSuffix.startsWith(trimmedCompletion)) {
		return
	}

	if (!shouldReturnMultiline) {
		if (completion.startsWith('\n')) {
			// TODO improve cache for this case so that we can use cache when accepting the first line of a multiline completion which starts with \n
			return completion.split('\n').slice(0, 2).join('\n') + '\n'
		} else if (completion.includes('\n')) {
			return completion.split('\n').slice(0, 1).join('\n') + '\n'
		}
	}

	return completion
}

export class Autocompletor {
	#lastTs = Date.now()
	#cache: LRUCache<number, CacheCompletion> = new LRUCache({
		max: 10
	})
	#scriptLang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json'
	#abortController: AbortController = new AbortController()
	#completionDisposable: IDisposable
	#cursorDisposable: IDisposable
	#lastTsCompletions: string[] = []
	#contextWindow: number = 0

	constructor(
		editor: meditor.IStandaloneCodeEditor,
		scriptLang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json'
	) {
		setGlobalCSS(
			'ai-chat-autocomplete',
			`
			.ai-completion-diff {
				background: var(--vscode-diffEditor-removedTextBackground);
			}
		`
		)
		this.#scriptLang = scriptLang

		const deletionsCues = editor.createDecorationsCollection()

		const completionModel = get(copilotInfo).codeCompletionModel
		this.#contextWindow = getModelContextWindow(completionModel?.model ?? '')

		this.#completionDisposable = languages.registerInlineCompletionsProvider(
			{ pattern: '**' },
			{
				provideInlineCompletions: async (model, position, context, token) => {
					if (
						token.isCancellationRequested ||
						model.uri.toString() !== editor.getModel()?.uri.toString()
					) {
						return { items: [] }
					}

					const shouldReturnMultiline = this.#shouldReturnMultiline(model, position)

					const result = await this.#autocomplete(model, position)

					if (!result) {
						return { items: [] }
					}

					const range = {
						startLineNumber: position.lineNumber,
						startColumn: position.column,
						endLineNumber: position.lineNumber,
						endColumn: position.column
					}

					const toEol = {
						...range,
						endColumn: model.getLineMaxColumn(position.lineNumber)
					}

					// if shouldReturnMultiline is false, only keep first line of a multiline completion (keeps final new line)
					let completion = filterCompletion(result.completion, result.suffix, shouldReturnMultiline)

					if (!completion) {
						return { items: [] }
					}

					// if completion ends with new line, we want the suggestion to replace the end of the current line
					const endsWithNewLine = completion.endsWith('\n')
					if (endsWithNewLine) {
						// remove new line
						completion = completion.slice(0, -1)
						// if suggestion === code to be replaced, do nothing
						const code = model.getValueInRange(toEol)
						if (completion === code) {
							return {
								items: []
							}
						}

						// set deletion cue for content that will be replaced by the suggestion
						deletionsCues.set([
							{
								range: toEol,
								options: {
									className: 'ai-completion-diff'
								}
							}
						])
					}

					const multiline = completion.indexOf('\n') !== -1
					return {
						items: [
							{
								insertText: completion,
								range: multiline ? toEol : range,
								filterText: completion,
								// if completion ends with new line, after applying the suggestion, delete the rest of the line
								additionalTextEdits:
									endsWithNewLine && !multiline
										? [
												{
													range: toEol,
													text: ''
												}
											]
										: []
							}
						]
					}
				},
				freeInlineCompletions: () => {}
			}
		)

		this.#cursorDisposable = editor.onDidChangeCursorPosition(async (e) => {
			deletionsCues.clear()
			if (e.source === 'mouse') {
				const model = editor.getModel()
				if (model) {
					this.#autocomplete(model, e.position)
				}
			}
		})
	}

	static isProviderModelSupported(providerModel: AIProviderModel | undefined) {
		return (
			providerModel &&
			providerModel.provider === 'mistral' &&
			providerModel.model.startsWith('codestral-') &&
			!providerModel.model.startsWith('codestral-embed')
		)
	}

	dispose() {
		this.#completionDisposable.dispose()
		this.#cursorDisposable.dispose()
	}

	#shouldReturnMultiline(model: meditor.ITextModel, position: Position) {
		if (position.column === model.getLineMaxColumn(position.lineNumber)) {
			const cachedCompletion = this.#cache.get(position.lineNumber)
			if (cachedCompletion) {
				const firstCachedLine =
					cachedCompletion.linePrefix + cachedCompletion.completion.split('\n')[0]
				const lineContent = model.getLineContent(position.lineNumber)
				return firstCachedLine === lineContent
			}
		}
		return false
	}

	#markersAtCursor(pos: Position, all: meditor.IMarker[]) {
		const padding = 1
		return all.filter(
			(m) =>
				m.startLineNumber >= pos.lineNumber - padding && m.endLineNumber <= pos.lineNumber + padding
		)
	}

	#partsToText(parts: { text: string; kind: string }[]) {
		return parts.map((p) => p.text).join('')
	}

	#formatCompletionEntry(details: {
		name: string
		displayParts: { text: string; kind: string }[]
		documentation: { kind: string; text: string }[]
		tags: { name: string; text: { kind: string; text: string }[] }[]
	}) {
		let ret = 'SIGNATURE: ' + this.#partsToText(details.displayParts) + '\n'
		for (const doc of details.documentation) {
			ret += 'DOC: ' + doc.text + '\n'
		}
		ret += 'TAGS: ' + JSON.stringify(details.tags)
		return ret
	}

	#formatHelp(help: {
		prefixDisplayParts: {
			text: string
			kind: string
		}[]
		suffixDisplayParts: {
			text: string
			kind: string
		}[]
		separatorDisplayParts: {
			text: string
			kind: string
		}[]
		documentation: {
			kind: string
			text: string
		}[]
		parameters: {
			displayParts: {
				text: string
				kind: string
			}[]
		}[]
	}) {
		const signature =
			this.#partsToText(help.prefixDisplayParts) +
			help.parameters
				.map((p) => this.#partsToText(p.displayParts))
				.join(this.#partsToText(help.separatorDisplayParts)) +
			this.#partsToText(help.suffixDisplayParts)

		const doc = this.#partsToText(help.documentation)

		let ret = 'SIGNATURE: ' + signature + '\n'
		ret += 'DOC: ' + doc + '\n'
		return ret
	}

	// get ts completitions when current word has a dot
	async #getTsCompletions(model: meditor.ITextModel, position: Position) {
		try {
			const offs = model.getOffsetAt(position)

			const workerFactory = await languages.typescript.getTypeScriptWorker()
			const worker = await workerFactory(model.uri)
			const info = await worker.getCompletionsAtPosition(model.uri.toString(), offs)

			const line = model.getLineContent(position.lineNumber)
			const word = line.substring(0, position.column)
			let hasDot = false
			let afterDot = ''

			let entries: string[] = []
			for (let i = word.length - 1; i >= 0; i--) {
				if (word[i] === ' ') {
					break
				}
				if (word[i] === '.') {
					hasDot = true
					afterDot = word.substring(i + 1).split('(')[0]
					break
				}
			}
			if (hasDot) {
				const filteredEntries = (info?.entries ?? []).filter((e: { name: string }) =>
					afterDot ? e?.name?.startsWith(afterDot) : true
				)
				const detailedEntries = await Promise.all(
					filteredEntries
						.slice(0, TYPE_SCRIPT_COMPLETIONS_DETAILS_MAX)
						.map((e: { name: string }) =>
							worker.getCompletionEntryDetails(model.uri.toString(), offs, e.name)
						)
				)
				entries.push(...detailedEntries.map((e) => this.#formatCompletionEntry(e)))
			}

			// get signature of open parenthesis
			const help = await worker.getSignatureHelpItems(model.uri.toString(), offs, {
				triggerReason: { kind: 'invoked' }
			})
			if (help && help.items?.length > 0) {
				entries.push(this.#formatHelp(help.items[0]))
			}
			return entries
		} catch (e) {
			console.error('Error getting ts completions', e)
			return []
		}
	}

	async #autocomplete(
		model: meditor.ITextModel,
		position: Position
	): Promise<{ completion: string; suffix: string } | undefined> {
		const thisTs = Date.now()
		this.#lastTs = thisTs

		await sleep(200)

		if (model.isDisposed()) {
			return
		}

		if (thisTs !== this.#lastTs) {
			return
		}

		const linePrefix = model.getValueInRange({
			startLineNumber: position.lineNumber,
			startColumn: 1,
			endLineNumber: position.lineNumber,
			endColumn: position.column
		})

		const suffix = model.getValueInRange({
			startLineNumber: position.lineNumber,
			startColumn: position.column,
			endLineNumber: model.getLineCount(),
			endColumn: model.getLineMaxColumn(model.getLineCount())
		})

		// get ts completions
		const tsCompletions =
			this.#scriptLang === 'bun' ? await this.#getTsCompletions(model, position) : []
		// reset cache for this line if new ts completions are available
		if (
			tsCompletions.length > 0 &&
			tsCompletions.some((c) => !this.#lastTsCompletions.includes(c))
		) {
			this.#cache.delete(position.lineNumber)
		}
		this.#lastTsCompletions = tsCompletions

		const cachedCompletion = this.#cache.get(position.lineNumber)
		if (cachedCompletion) {
			if (
				position.column > cachedCompletion.column &&
				linePrefix.length < cachedCompletion.linePrefix.length + cachedCompletion.completion.length
			) {
				const completeLine = cachedCompletion.linePrefix + cachedCompletion.completion
				const newLinePrefix = completeLine.substring(0, position.column - 1)
				if (newLinePrefix === linePrefix) {
					const modifiedCompletion = cachedCompletion.completion.slice(
						position.column - cachedCompletion.column
					)
					return { completion: modifiedCompletion, suffix }
				}
			} else if (
				position.column === cachedCompletion.column &&
				cachedCompletion.linePrefix === linePrefix
			) {
				return { completion: cachedCompletion.completion, suffix }
			}
		}

		this.#abortController.abort()
		this.#abortController = new AbortController()
		const prefix = model.getValueInRange({
			startLineNumber: 1,
			startColumn: 1,
			endLineNumber: position.lineNumber,
			endColumn: position.column
		})

		const markers = meditor.getModelMarkers({ resource: model.uri })
		const markersAtCursor = this.#markersAtCursor(position, markers)

		const librariesCompletions: string = tsCompletions
			.join('\n')
			.slice(0, Math.floor(this.#contextWindow * TYPE_SCRIPT_COMPLETIONS_MAX_RATIO))

		const completion = await autocompleteRequest(
			{
				prefix,
				suffix,
				scriptLang: this.#scriptLang,
				markers: markersAtCursor,
				libraries: librariesCompletions
			},
			this.#abortController
		)

		if (!completion) {
			return
		}

		this.#cache.set(position.lineNumber, {
			linePrefix,
			completion,
			column: position.column
		})

		return { completion, suffix }
	}
}
