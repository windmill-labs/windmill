<script lang="ts" context="module">
	import * as monaco from 'monaco-editor'

	monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
		target: monaco.languages.typescript.ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noLib: true
	})

	monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
		validate: true,
		allowComments: false,
		schemas: [],
		enableSchemaRequest: true
	})
</script>

<script lang="ts">
	import { browser, dev } from '$app/env'

	import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
	import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
	import yamlWorker from 'monaco-yaml/yaml.worker?worker'
	import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

	import { buildWorkerDefinition } from 'monaco-editor-workers'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import { createHash, editorConfig, langToExt, updateOptions } from '$lib/editorUtils'

	let divEl: HTMLDivElement | null = null
	let editor: monaco.editor.IStandaloneCodeEditor
	let model: monaco.editor.ITextModel

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

	const dispatch = createEventDispatcher()

	const uri = `file:///${hash}.${langToExt(lang)}`

	if (browser) {
		if (dev) {
			buildWorkerDefinition(
				'../../../node_modules/monaco-editor-workers/dist/workers',
				import.meta.url,
				false
			)
		} else {
			// @ts-ignore
			self.MonacoEnvironment = {
				getWorker: function (_moduleId: any, label: string) {
					if (label === 'json') {
						return new jsonWorker()
					} else if (label === 'yaml') {
						return new yamlWorker()
					} else if (label === 'typescript' || label === 'javascript') {
						return new tsWorker()
					} else {
						return new editorWorker()
					}
				}
			}
		}
	}

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

	function format() {
		if (editor) {
			code = getCode()
			editor.getAction('editor.action.formatDocument').run()
			if (formatAction) {
				formatAction()
				code = getCode()
			}
		}
	}

	let width = 0
	async function loadMonaco() {
		model = monaco.editor.createModel(code, lang, monaco.Uri.parse(uri))

		model.updateOptions(updateOptions)

		editor = monaco.editor.create(
			divEl as HTMLDivElement,
			editorConfig(model, code, lang, automaticLayout, fixedOverflowWidgets)
		)

		let timeoutModel: NodeJS.Timeout | undefined = undefined
		editor.onDidChangeModelContent((event) => {
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				code = getCode()
				dispatch('change', { code })
			}, 200)
		})

		if (autoHeight) {
			let ignoreEvent = false
			const updateHeight = () => {
				const contentHeight = Math.min(1000, editor.getContentHeight())
				if (divEl) {
					divEl.style.height = `${contentHeight}px`
				}
				try {
					ignoreEvent = true
					editor.layout({ width, height: contentHeight })
				} finally {
					ignoreEvent = false
				}
			}
			editor.onDidContentSizeChange(updateHeight)
			updateHeight()
		}

		editor.onDidFocusEditorText(() => {
			editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, function () {
				code = getCode()
				shouldBindKey && format && format()
			})

			editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, function () {
				code = getCode()
				shouldBindKey && cmdEnterAction && cmdEnterAction()
			})
			dispatch('focus')
		})

		editor.onDidBlurEditorText(() => {
			code = getCode()
			dispatch('blur')
		})

		if (lang == 'javascript' && extraLib != '') {
			monaco.languages.typescript.javascriptDefaults.setExtraLibs([
				{
					content: extraLib,
					filePath: 'windmill.d.ts'
				}
			])
		} else {
			monaco.languages.typescript.javascriptDefaults.setExtraLibs([])
		}
	}

	onMount(() => {
		if (browser) {
			loadMonaco()
		}
	})

	onDestroy(() => {
		try {
			model && model.dispose()
			editor && editor.dispose()
		} catch (err) {}
	})
</script>

<div bind:this={divEl} class="{$$props.class} editor" bind:clientWidth={width} />

<style>
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
