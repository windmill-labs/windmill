import { initServices } from 'monaco-languageclient/vscode/services'
// import getThemeServiceOverride from '@codingame/monaco-vscode-theme-service-override'
// import getTextmateServiceOverride from '@codingame/monaco-vscode-textmate-service-override'
import getMonarchServiceOverride from '@codingame/monaco-vscode-monarch-service-override'
import '@codingame/monaco-vscode-standalone-typescript-language-features'
import { editor as meditor } from 'monaco-editor/esm/vs/editor/editor.api'

export let isInitialized = false
export let isInitializing = false

export async function initializeVscode(caller?: string) {
	if (!isInitialized && !isInitializing) {
		console.log(`Initializing vscode-api from ${caller ?? 'unknown'}`)
		isInitializing = true

		try {
			// init vscode-api
			await initServices({
				serviceConfig: {
					userServices: {
						// ...getThemeServiceOverride(),
						// ...getTextmateServiceOverride()
						...getMonarchServiceOverride()
					},
					debugLogging: true,
					enableExtHostWorker: false
				}
			})
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
						foreground: '808b9f',
						token: 'comment'
					},
					{
						foreground: 'a3be8c',
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
						foreground: '81a1c1',
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
					}
				],
				colors: {
					'editor.foreground': '#D8DEE9',
					'editor.background': '#272D38',
					'editor.selectionBackground': '#434C5ECC',
					'editor.lineHighlightBackground': '#3B4252',
					'editorCursor.foreground': '#D8DEE9',
					'editorWhitespace.foreground': '#434C5ECC'
				}
			})

			meditor.defineTheme('myTheme', {
				base: 'vs',
				inherit: true,
				rules: [],
				colors: {
					'editorLineNumber.foreground': '#999',
					'editorGutter.background': '#F9FAFB'
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
