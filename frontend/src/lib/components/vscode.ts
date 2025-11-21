import '@codingame/monaco-vscode-standalone-typescript-language-features'

import { editor as meditor, Uri as mUri } from 'monaco-editor'

export let isInitialized = false
export let isInitializing = false

import {
	getEnhancedMonacoEnvironment,
	MonacoVscodeApiWrapper
} from 'monaco-languageclient/vscodeApiWrapper'
import getLanguagesServiceOverride from '@codingame/monaco-vscode-languages-service-override'
import { getCssColor } from '$lib/utils'

export function buildWorkerDefinition() {
	const envEnhanced = getEnhancedMonacoEnvironment()

	const getWorker = (moduleId: string, label: string) => {
		console.log(`getWorker: moduleId: ${moduleId} label: ${label}`)

		let selector = label

		// const defaultTextEditorWorker = () => new Worker(
		// 	new URL('@codingame/monaco-vscode-editor-api/esm/vs/editor/editor.worker.js', import.meta.url),
		// 	{ type: 'module' }
		// );
		// const defaultTextMateWorker = () => new Worker(
		// 	new URL('@codingame/monaco-vscode-textmate-service-override/worker', import.meta.url),
		// 	{ type: 'module' }
		// );

		let workerLoaders = {
			TextEditorWorker: () => {
				return new Worker(
					new URL(
						'@codingame/monaco-vscode-editor-api/esm/vs/editor/editor.worker.js',
						import.meta.url
					),
					{
						type: 'module'
					}
				)
			},
			// javascript: () => {
			// 	return new Worker(new URL('monaco-editor-wrapper/workers/module/ts', import.meta.url), {
			// 		type: 'module'
			// 	})
			// },
			javascript: () => {
				return new Worker(
					new URL(
						'@codingame/monaco-vscode-standalone-typescript-language-features/worker',
						import.meta.url
					),
					{
						type: 'module'
					}
				)
			},
			typescript: () => {
				return new Worker(
					new URL(
						'@codingame/monaco-vscode-standalone-typescript-language-features/worker',
						import.meta.url
					),
					{
						type: 'module'
					}
				)
			},
			json: () => {
				return new Worker(
					new URL(
						'@codingame/monaco-vscode-standalone-json-language-features/worker',
						import.meta.url
					),
					{
						type: 'module'
					}
				)
			},
			html: () => {
				return new Worker(
					new URL(
						'@codingame/monaco-vscode-standalone-html-language-features/worker',
						import.meta.url
					),
					{
						type: 'module'
					}
				)
			},
			css: () => {
				return new Worker(
					new URL(
						'@codingame/monaco-vscode-standalone-css-language-features/worker',
						import.meta.url
					),
					{
						type: 'module'
					}
				)
			},
			graphql: () => {
				console.log('Creating graphql worker')
				return new Worker(new URL(`../monaco_workers/graphql.worker.bundle.js`, import.meta.url), {
					name: 'graphql'
				})
			}
		}
		const workerFunc = workerLoaders[selector]
		if (workerFunc !== undefined) {
			return workerFunc()
		} else {
			throw new Error(`Unimplemented worker ${label} (${moduleId})`)
		}
	}
	envEnhanced.getWorker = getWorker
}

export async function initializeVscode(caller?: string, htmlContainer?: HTMLElement) {
	if (!isInitialized && !isInitializing) {
		console.log(`Initializing vscode-api from ${caller ?? 'unknown'}`)
		isInitializing = true

		try {
			// init vscode-api
			const apiWrapper = new MonacoVscodeApiWrapper({
				$type: 'classic',
				viewsConfig: {
					$type: 'EditorService'
				},
				serviceOverrides: {
					// ...getLogServiceOverride()
					// ...getThemeServiceOverride(),
					// ...getTextmateServiceOverride()
					// ...getConfigurationServiceOverride(),
					// ...getKeybindingsServiceOverride()
					...getLanguagesServiceOverride()
				},
				userConfiguration: {
					json: JSON.stringify({
						'editor.experimental.asyncTokenization': true
					})
				},
				advanced: {
					enableExtHostWorker: true
				},
				monacoWorkerFactory: buildWorkerDefinition
			})
			await apiWrapper.start()

			isInitialized = true
			meditor.defineTheme('nord', {
				base: 'vs-dark',
				inherit: true,
				rules: [
					{
						background: '2E3440',
						token: ''
					},
					{
						foreground: '98A7C1',
						token: 'comment'
					},
					{
						foreground: 'b3deac',
						token: 'string'
					},
					{
						foreground: 'b48ead',
						token: 'constant.numeric'
					},
					{
						foreground: '81a1c1',
						token: 'constant.language'
					},
					{
						foreground: '92bae6',
						token: 'keyword'
					},
					{
						foreground: '81a1c1',
						token: 'storage'
					},
					{
						foreground: '81a1c1',
						token: 'storage.type'
					},
					{
						foreground: '8fbcbb',
						token: 'entity.name.class'
					},
					{
						foreground: '8fbcbb',
						fontStyle: '  bold',
						token: 'entity.other.inherited-class'
					},
					{
						foreground: '88c0d0',
						token: 'entity.name.function'
					},
					{
						foreground: '81a1c1',
						token: 'entity.name.tag'
					},
					{
						foreground: '8fbcbb',
						token: 'entity.other.attribute-name'
					},
					{
						foreground: '88c0d0',
						token: 'support.function'
					},
					{
						foreground: 'f8f8f0',
						background: 'f92672',
						token: 'invalid'
					},
					{
						foreground: 'f8f8f0',
						background: 'ae81ff',
						token: 'invalid.deprecated'
					},
					{
						foreground: 'b48ead',
						token: 'constant.color.other.rgb-value'
					},
					{
						foreground: 'ebcb8b',
						token: 'constant.character.escape'
					},
					{
						foreground: '8fbcbb',
						token: 'variable.other.constant'
					},
					{ token: 'string.value.json', foreground: 'e8b886' }, // string values in JSON
					{ token: 'keyword.json', foreground: 'e8b886' } // true, false, null in JSON
				],
				colors: {
					'editor.foreground': '#D8DEE9',
					'editor.background': getCssColor('surface-input', { format: 'hex-dark' }),
					'editor.selectionBackground': '#515A6D',
					'editor.inactiveSelectionBackground': '#515A6DB0',
					'editor.lineHighlightBackground': '#3B4252',
					'editorCursor.foreground': '#D8DEE9',
					'editorWhitespace.foreground': '#515A6D',
					'editorIndentGuide.background1': '#5A647860',
					'editorIndentGuide.activeBackground1': '#6A7488',
					'editorLineNumber.foreground': '#515A6D',
					'editorLineNumber.activeForeground': '#D8DEE980'
				}
			})

			meditor.defineTheme('myTheme', {
				base: 'vs',
				inherit: true,
				rules: [],
				colors: {
					'editor.background': '#FFFFFF',
					'editor.foreground': '#2d3748',
					'editorLineNumber.foreground': '#C2C9D1',
					'editorLineNumber.activeForeground': '#989DA5',
					'editorGutter.background': '#FFFFFF00'
				}
			})

			if (document.documentElement.classList.contains('dark')) {
				meditor.setTheme('nord')
			} else {
				meditor.setTheme('myTheme')
			}
		} catch (e) {
			console.error('Failed to initialize monaco services', e)
		} finally {
			isInitialized = true
			isInitializing = false
		}
	} else {
		while (isInitializing && !isInitialized) {
			console.log('Waiting for initialization of monaco services')
			await new Promise((resolve) => setTimeout(resolve, 100))
		}
	}
}

export function keepModelAroundToAvoidDisposalOfWorkers() {
	const keepEditorUri = mUri.parse('file:///avoidDisposalOfWorkers')
	if (!meditor?.getModel(keepEditorUri)) {
		meditor.createModel('', 'typescript', keepEditorUri)
	}
}

export let MONACO_Y_PADDING = 6.5
