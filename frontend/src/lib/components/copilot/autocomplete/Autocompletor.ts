import type { AIProviderModel, ScriptLang } from '$lib/gen'
import { sleep } from '$lib/utils'
import { Position, type editor as meditor, languages, type IDisposable } from 'monaco-editor'
import { LRUCache } from 'lru-cache'
import { autocompleteRequest } from './request'
import { FIM_MAX_TOKENS } from '../lib'

type CacheCompletion = {
	linePrefix: string
	completion: string
	column: number
}

function filterCompletion(completion: string, suffix: string): string | undefined {
	const trimmedCompletion = completion.replaceAll('\n', '')
	const trimmedSuffix = suffix.slice(0, FIM_MAX_TOKENS).replaceAll('\n', '')

	if (trimmedSuffix.startsWith(trimmedCompletion)) {
		console.log('suffix starts with completion', suffix, completion)
		return
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

	constructor(
		editor: meditor.IStandaloneCodeEditor,
		scriptLang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json'
	) {
		this.#scriptLang = scriptLang

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
					const result = await this.#autocomplete(model, position)

					if (result) {
						const completion = filterCompletion(result.completion, result.suffix)

						if (!completion) {
							return { items: [] }
						}

						let range = {
							startLineNumber: position.lineNumber,
							startColumn: position.column,
							endLineNumber: position.lineNumber,
							endColumn: position.column
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
									range
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
					console.debug('autocomplete partial cache hit', modifiedCompletion)
					return { completion: modifiedCompletion, suffix }
				}
			} else if (
				position.column === cachedCompletion.column &&
				cachedCompletion.linePrefix === linePrefix
			) {
				console.debug('autocomplete exact cache hit', cachedCompletion.completion)
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

		const completion = await autocompleteRequest(
			{
				prefix,
				suffix,
				scriptLang: this.#scriptLang
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
