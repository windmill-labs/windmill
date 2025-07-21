import type { AIProviderModel, ScriptLang } from '$lib/gen'
import { sleep } from '$lib/utils'
import { editor as meditor, Position, languages, type IDisposable, Range } from 'monaco-editor'
import { LRUCache } from 'lru-cache'
import { autocompleteRequest } from './request'
import { FIM_MAX_TOKENS, getModelContextWindow } from '../lib'
import { setGlobalCSS } from '../shared'
import { get } from 'svelte/store'
import { copilotInfo } from '$lib/stores'

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
			return completion.split('\n').slice(0, 2).join('\n')
		} else {
			return completion.split('\n').slice(0, 1).join('\n')
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
	#markers: meditor.IMarker[] = []
	#libraries: { code: string; path: string }[] = []

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

					if (result) {
						const completion = filterCompletion(
							result.completion,
							result.suffix,
							shouldReturnMultiline
						)

						if (!completion) {
							return { items: [] }
						}

						let range = {
							startLineNumber: position.lineNumber,
							startColumn: position.column,
							endLineNumber: position.lineNumber,
							endColumn: position.column
						}

						// if completion takes whole line, delete the rest of the line after the suggestion
						const isWholeLine = result.completion.indexOf('\n') !== -1
						if (isWholeLine) {
							// if code between position.column and the end of the line is the same as the completion, return empty items
							const code = model.getValueInRange({
								startLineNumber: position.lineNumber,
								startColumn: position.column,
								endLineNumber: position.lineNumber,
								endColumn: model.getLineMaxColumn(position.lineNumber)
							})
							if (completion === code) {
								return {
									items: []
								}
							}

							const toEol = new Range(
								position.lineNumber,
								position.column,
								position.lineNumber,
								model.getLineMaxColumn(position.lineNumber)
							)

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
						if (multiline) {
							// if multiline the range should span until the end of the line
							range.endColumn = model.getLineMaxColumn(position.lineNumber)
						}

						return {
							items: [
								{
									insertText: completion,
									range,
									additionalTextEdits: isWholeLine
										? [
												{
													range: {
														startLineNumber: position.lineNumber,
														startColumn: position.column,
														endLineNumber: position.lineNumber,
														endColumn: model.getLineMaxColumn(position.lineNumber)
													},
													text: ''
												}
											]
										: []
								}
							]
						}
					} else {
						return {
							items: []
						}
					}
				},
				freeInlineCompletions: () => {}
			}
		)

		this.#cursorDisposable = editor.onDidChangeCursorPosition(async (e) => {
			deletionsCues.clear()
			const model = editor.getModel()
			if (model) {
				const markers = meditor.getModelMarkers({ resource: model.uri })
				const hits = this.#markersAtCursor(e.position, markers)
				this.#markers = hits
				if (e.source === 'mouse') {
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

	addLibrary(code: string, path: string) {
		this.#libraries.push({ code, path })
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
		const lineBeforeCount = 1
		return all.filter(
			(m) =>
				m.startLineNumber >= pos.lineNumber - lineBeforeCount && m.endLineNumber <= pos.lineNumber
		)
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

		const completionModel = get(copilotInfo).codeCompletionModel
		const contextWindow = getModelContextWindow(completionModel?.model ?? '')
		const librariesLimitedCode: string = this.#libraries
			.filter(
				(l) =>
					!l.path.includes('windmill') &&
					!l.path.includes('package.json') &&
					!l.path.includes('bun-types')
			)
			.map((l) => l.code)
			.join('\n')
			.slice(0, Math.floor(contextWindow * 0.1))

		const completion = await autocompleteRequest(
			{
				prefix,
				suffix,
				scriptLang: this.#scriptLang,
				markers: this.#markers,
				libraries: librariesLimitedCode
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
