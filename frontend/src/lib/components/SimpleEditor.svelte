<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { createHash, editorConfig, langToExt, updateOptions } from '$lib/editorUtils'
	import {
		editor as meditor,
		KeyCode,
		KeyMod,
		Uri as mUri,
		languages,
		type IRange
	} from 'monaco-editor'
	import * as monaco from 'monaco-editor'
	import 'monaco-editor/esm/vs/basic-languages/sql/sql.contribution'
	import 'monaco-editor/esm/vs/basic-languages/yaml/yaml.contribution'
	import 'monaco-editor/esm/vs/basic-languages/typescript/typescript.contribution'
	import 'monaco-editor/esm/vs/basic-languages/javascript/javascript.contribution'
	import 'monaco-editor/esm/vs/basic-languages/graphql/graphql.contribution'
	import 'monaco-editor/esm/vs/language/json/monaco.contribution'
	import 'monaco-editor/esm/vs/language/typescript/monaco.contribution'
	import 'monaco-editor/esm/vs/basic-languages/css/css.contribution'
	import 'monaco-editor/esm/vs/language/css/monaco.contribution'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'

	import libStdContent from '$lib/es5.d.ts.txt?raw'
	import { buildWorkerDefinition } from './build_workers'
	import { initializeVscode } from './vscode'
	import EditorTheme from './EditorTheme.svelte'
	// import { createConfiguredEditor } from 'vscode/monaco'
	// import type { IStandaloneCodeEditor } from 'vscode/vscode/vs/editor/standalone/browser/standaloneCodeEditor'

	import {
		configureMonacoTailwindcss,
		tailwindcssData,
		type TailwindConfig
	} from 'monaco-tailwindcss'

	let divEl: HTMLDivElement | null = null
	let editor: meditor.IStandaloneCodeEditor
	let model: meditor.ITextModel

	export let lang: string
	export let code: string = ''
	export let hash: string = createHash()
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let extraLib: string = ''
	export let shouldBindKey: boolean = true
	export let autoHeight = false
	export let fixedOverflowWidgets = true
	export let small = false

	const dispatch = createEventDispatcher()

	const uri = `file:///${hash}.${langToExt(lang)}`

	buildWorkerDefinition('../../../workers', import.meta.url, false)

	export function getCode(): string {
		return editor?.getValue() ?? ''
	}

	export function insertAtCursor(code: string): void {
		if (editor) {
			editor.trigger('keyboard', 'type', { text: code })
		}
	}

	export function setCode(ncode: string): void {
		code = ncode
		if (editor) {
			editor.setValue(ncode)
		}
	}

	export function format() {
		if (editor) {
			code = getCode()
			editor.getAction('editor.action.formatDocument')?.run()
			if (formatAction) {
				formatAction()
				code = getCode()
			}
		}
	}

	export function focus() {
		editor?.focus()
	}

	export function getSelectedLines(): string | undefined {
		if (editor) {
			const selection = editor.getSelection()
			if (selection) {
				const range: IRange = {
					startLineNumber: selection.startLineNumber,
					startColumn: 1,
					endLineNumber: selection.endLineNumber + 1,
					endColumn: 1
				}
				return editor.getModel()?.getValueInRange(range)
			}
		}
	}

	export function onDidChangeCursorSelection(f: (e: meditor.ICursorSelectionChangedEvent) => void) {
		if (editor) {
			return editor.onDidChangeCursorSelection(f)
		}
	}

	export function show(): void {
		divEl?.classList.remove('hidden')
	}

	export function hide(): void {
		divEl?.classList.add('hidden')
	}

	let width = 0
	let initialized = false
	async function loadMonaco() {
		await initializeVscode()
		initialized = true
		languages.typescript.javascriptDefaults.setCompilerOptions({
			target: languages.typescript.ScriptTarget.Latest,
			allowNonTsExtensions: true,
			noLib: true
		})

		const tailwindConfig: TailwindConfig = {
			theme: {
				extend: {
					screens: {
						television: '90000px'
					},
					spacing: {
						'128': '32rem'
					},
					colors: {
						// https://icolorpalette.com/color/molten-lava
						lava: '#b5332e',
						// Taken from https://icolorpalette.com/color/ocean-blue
						ocean: {
							50: '#f2fcff',
							100: '#c1f2fe',
							200: '#90e9ff',
							300: '#5fdfff',
							400: '#2ed5ff',
							500: '#00cafc',
							600: '#00a3cc',
							700: '#007c9b',
							800: '#00546a',
							900: '#002d39'
						}
					}
				}
			}
		}

		configureMonacoTailwindcss(monaco, { tailwindConfig })

		monaco.languages.css.cssDefaults.setOptions({
			data: {
				dataProviders: {
					tailwindcssData
				}
			}
		})

		languages.json.jsonDefaults.setDiagnosticsOptions({
			validate: true,
			allowComments: false,
			schemas: [],
			enableSchemaRequest: true
		})

		try {
			model = meditor.createModel(code, lang, mUri.parse(uri))
		} catch (err) {
			console.log('model already existed', err)
			const nmodel = meditor.getModel(mUri.parse(uri))
			if (!nmodel) {
				throw err
			}
			model = nmodel
		}
		model.updateOptions(updateOptions)
		let widgets: HTMLElement | undefined =
			document.getElementById('monaco-widgets-root') ?? undefined

		editor = meditor.create(divEl as HTMLDivElement, {
			...editorConfig(code, lang, automaticLayout, fixedOverflowWidgets),
			model,
			overflowWidgetsDomNode: widgets,
			fontSize: small ? 12 : 14
		})

		let timeoutModel: NodeJS.Timeout | undefined = undefined
		editor.onDidChangeModelContent((event) => {
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				code = getCode()
				dispatch('change', { code })
			}, 200)
		})

		editor.onDidFocusEditorText(() => {
			dispatch('focus')

			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {
				code = getCode()
				shouldBindKey && format && format()
			})
		})

		if (autoHeight) {
			const updateHeight = () => {
				const contentHeight = Math.min(1000, editor.getContentHeight())
				if (divEl) {
					divEl.style.height = `${contentHeight}px`
				}
				try {
					editor.layout({ width, height: contentHeight })
				} finally {
				}
			}
			editor.onDidContentSizeChange(updateHeight)
			updateHeight()
		}

		editor.onDidFocusEditorText(() => {
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {
				code = getCode()
				shouldBindKey && format && format()
			})

			editor.addCommand(KeyMod.CtrlCmd | KeyCode.Enter, function () {
				code = getCode()
				shouldBindKey && cmdEnterAction && cmdEnterAction()
			})
			dispatch('focus')
		})

		editor.onDidBlurEditorText(() => {
			code = getCode()
		})
	}

	//$: lang == 'css' && initialized && addCSSClassCompletions()

	function loadExtraLib() {
		if (lang == 'javascript') {
			const stdLib = { content: libStdContent, filePath: 'es5.d.ts' }
			if (extraLib != '') {
				languages.typescript.javascriptDefaults.setExtraLibs([
					{
						content: extraLib,
						filePath: 'windmill.d.ts'
					},
					stdLib
				])
			} else {
				languages.typescript.javascriptDefaults.setExtraLibs([stdLib])
			}
		}
	}

	let mounted = false
	onMount(async () => {
		if (BROWSER) {
			mounted = true
			await loadMonaco()
		}
	})

	$: mounted && extraLib && initialized && loadExtraLib()

	onDestroy(() => {
		try {
			model && model.dispose()
			editor && editor.dispose()
		} catch (err) {}
	})
</script>

<EditorTheme />

<div bind:this={divEl} class="{$$props.class ?? ''} editor" bind:clientWidth={width} />

<style lang="postcss">
	.editor {
		@apply rounded-lg p-0;
	}

	.small-editor {
		/* stylelint-disable-next-line unit-allowed-list */
		height: 26vh;
	}

	.few-lines-editor {
		/* stylelint-disable-next-line unit-allowed-list */
		height: 100px;
	}
</style>
