<script lang="ts">
	import { browser, dev } from '$app/environment'

	import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
	import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
	import yamlWorker from 'monaco-yaml/yaml.worker?worker'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import { createHash, editorConfig, langToExt, updateOptions } from '$lib/editorUtils'
	import {
		languages,
		editor as meditor,
		KeyCode,
		KeyMod,
		Uri as mUri
	} from 'monaco-editor/esm/vs/editor/editor.main'

	meditor.defineTheme('myTheme', {
		base: 'vs',
		inherit: true,
		rules: [],
		colors: {
			'editorLineNumber.foreground': '#999',
			'editorGutter.background': '#F9FAFB'
		}
	})
	meditor.setTheme('myTheme')

	languages.json.jsonDefaults.setDiagnosticsOptions({
		validate: true,
		allowComments: false,
		schemas: [],
		enableSchemaRequest: true
	})

	let divEl: HTMLDivElement | null = null
	let editor: meditor.IStandaloneCodeEditor
	let model: meditor.ITextModel

	export let lang: string
	export let code: string = ''
	export let hash: string = createHash()
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let shouldBindKey: boolean = true
	export let autoHeight = false
	export let fixedOverflowWidgets = true

	const dispatch = createEventDispatcher()

	const uri = `file:///${hash}.${langToExt(lang)}`

	if (browser) {
		if (!dev) {
			// @ts-ignore
			self.MonacoEnvironment = {
				getWorker: function (_moduleId: any, label: string) {
					if (label === 'json') {
						return new jsonWorker()
					} else if (label === 'yaml') {
						return new yamlWorker()
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
		model = meditor.createModel(code, lang, mUri.parse(uri))

		model.updateOptions(updateOptions)

		editor = meditor.create(
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

		editor.onDidFocusEditorText(() => {
			dispatch('focus')

			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {
				code = getCode()
				shouldBindKey && format && format()
			})
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
			dispatch('blur')
		})
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
