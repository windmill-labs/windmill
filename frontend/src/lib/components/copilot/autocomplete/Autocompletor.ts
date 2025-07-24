import type { AIProviderModel, ScriptLang } from '$lib/gen'
import { sleep } from '$lib/utils'
import { editor as meditor, Position, languages, type IDisposable } from 'monaco-editor'
import { LRUCache } from 'lru-cache'
import { autocompleteRequest } from './request'
import { FIM_MAX_TOKENS, getModelContextWindow } from '../lib'
import { setGlobalCSS } from '../shared'
import { get } from 'svelte/store'
import { copilotInfo } from '$lib/stores'
import type { MonacoLanguageClient } from 'monaco-languageclient'

// max ratio of completions to context window
const COMPLETIONS_MAX_RATIO = 0.1

// hard limit to max number of completions to fetch details for, to avoid performance overhead
const MAX_COMPLETIONS_DETAILS = 50

type CacheCompletion = {
	linePrefix: string
	completion: string
	column: number
}

type LanguageClientHelp = {
	signatures: {
		label: string
		documentation: { kind: string; value: string }
	}[]
}

type LanguageClientCompletion = {
	items: {
		label: string
	}[]
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
	#lastCompletions: string[] = []
	#contextWindow: number = 0
	#shouldShowDeletionCue = false
	#languageClient: MonacoLanguageClient | undefined = undefined

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
				handleItemDidShow: (items) => {
					const item = items.items[0]
					const model = editor.getModel()
					if (!item || !item.range || !model) {
						return
					}
					const toEol = {
						...item.range,
						endColumn: model.getLineMaxColumn(item.range.startLineNumber)
					}
					if (this.#shouldShowDeletionCue) {
						deletionsCues.set([
							{
								range: toEol,
								options: {
									className: 'ai-completion-diff'
								}
							}
						])
						this.#shouldShowDeletionCue = false
					}
				},
				provideInlineCompletions: async (model, position, context, token) => {
					if (
						token.isCancellationRequested ||
						model.uri.toString() !== editor.getModel()?.uri.toString()
					) {
						return { items: [] }
					}
					this.#shouldShowDeletionCue = false

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

						// set deletion cue for content that will be replaced by the suggestion
						if (!completion.includes('\n')) {
							this.#shouldShowDeletionCue = true
						}
					}

					const multiline = completion.indexOf('\n') !== -1
					return {
						items: [
							{
								insertText: completion,
								range: !endsWithNewLine && multiline ? toEol : range,
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
			this.#shouldShowDeletionCue = false
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

	setLanguageClient(client: MonacoLanguageClient) {
		this.#languageClient = client
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

	#formatTsHelp(help: {
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

	#formatLanguageClientHelp(help: LanguageClientHelp) {
		return help.signatures
			.map((s) => 'SIGNATURE: ' + s.label + '\n' + 'DOC: ' + s.documentation.value)
			.join('\n')
	}

	async #getCompletions(model: meditor.ITextModel, position: Position): Promise<string[]> {
		try {
			const line = model.getLineContent(position.lineNumber)
			const word = line.substring(0, position.column)
			let hasDot = false
			let afterDot = ''

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
				if (this.#scriptLang === 'bun') {
					return await this.#getTsCompletions(model, position, afterDot)
				} else {
					return await this.#getLanguageClientCompletions(model, position, afterDot)
				}
			}
			return []
		} catch (e) {
			console.error('Error getting completions', e)
			return []
		}
	}

	async #getTsCompletions(
		model: meditor.ITextModel,
		position: Position,
		afterDot: string
	): Promise<string[]> {
		try {
			const offs = model.getOffsetAt(position)

			const workerFactory = await languages.typescript.getTypeScriptWorker()
			const worker = await workerFactory(model.uri)
			const info = await worker.getCompletionsAtPosition(model.uri.toString(), offs)

			let entries: string[] = []
			const filteredEntries = (info?.entries ?? []).filter((e: { name: string }) =>
				afterDot ? e?.name?.startsWith(afterDot) : true
			)
			const detailedEntries = await Promise.all(
				filteredEntries
					.slice(0, MAX_COMPLETIONS_DETAILS)
					.map((e: { name: string }) =>
						worker.getCompletionEntryDetails(model.uri.toString(), offs, e.name)
					)
			)
			entries.push(...detailedEntries.map((e) => this.#formatCompletionEntry(e)))

			// get signature of open parenthesis
			const help = await worker.getSignatureHelpItems(model.uri.toString(), offs, {
				triggerReason: { kind: 'invoked' }
			})
			if (help && help.items?.length > 0) {
				entries.push(this.#formatTsHelp(help.items[0]))
			}
			return entries
		} catch (e) {
			console.error('Error getting ts completions', e)
			return []
		}
	}

	async #getLanguageClientCompletions(
		model: meditor.ITextModel,
		position: Position,
		afterDot: string
	): Promise<string[]> {
		try {
			if (!this.#languageClient) {
				return []
			}
			let entries: string[] = []
			const completions: LanguageClientCompletion = await this.#languageClient.sendRequest(
				'textDocument/completion',
				{
					textDocument: { uri: model.uri.toString() },
					position: {
						line: position.lineNumber - 1,
						character: position.column - 1
					}
				}
			)

			// if we failed to resolve a completion, don't try to resolve any more
			let failedToResolve = false
			const detailedEntries = await Promise.all(
				completions.items
					.filter((item) => (afterDot ? item.label.startsWith(afterDot) : true))
					.slice(0, MAX_COMPLETIONS_DETAILS)
					.map(async (item) => {
						try {
							if (failedToResolve) {
								return ''
							}
							const resolvedItem = await this.#languageClient!.sendRequest(
								'completionItem/resolve',
								item
							)
							return this.#formatLanguageClientHelp({
								signatures: [resolvedItem]
							} as LanguageClientHelp)
						} catch (e) {
							console.error('Failed to resolve completion item:', e)
							failedToResolve = true
							return ''
						}
					})
			)

			entries.push(...detailedEntries.filter((e) => e !== ''))

			const help: LanguageClientHelp = await this.#languageClient.sendRequest(
				'textDocument/signatureHelp',
				{
					textDocument: { uri: model.uri.toString() },
					position: {
						line: position.lineNumber - 1, // LSP uses 0-based line numbers
						character: position.column - 1 // LSP uses 0-based character positions
					}
				}
			)
			if (help && help.signatures.length > 0) {
				entries.push(this.#formatLanguageClientHelp(help))
			}
			return entries
		} catch (e) {
			console.error('Error getting language client completions', e)
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

		const completions = await this.#getCompletions(model, position)
		// reset cache for this line if new completions are available
		if (completions.length > 0 && completions.some((c) => !this.#lastCompletions.includes(c))) {
			this.#cache.delete(position.lineNumber)
		}
		this.#lastCompletions = completions

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

		const librariesCompletions: string = completions
			.join('\n')
			.slice(0, Math.floor(this.#contextWindow * COMPLETIONS_MAX_RATIO))

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
