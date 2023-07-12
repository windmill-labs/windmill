<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { onMount } from 'svelte'

	import 'monaco-editor/esm/vs/editor/edcore.main'
	import { editor as meditor } from 'monaco-editor/esm/vs/editor/editor.api'
	import 'monaco-editor/esm/vs/basic-languages/python/python.contribution'
	import 'monaco-editor/esm/vs/basic-languages/go/go.contribution'
	import 'monaco-editor/esm/vs/basic-languages/shell/shell.contribution'
	import 'monaco-editor/esm/vs/basic-languages/typescript/typescript.contribution'
	import 'monaco-editor/esm/vs/basic-languages/sql/sql.contribution'
	import 'monaco-editor/esm/vs/language/typescript/monaco.contribution'
	import { Breakpoints } from './apps/gridUtils'

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

	export let automaticLayout = true
	export let fixedOverflowWidgets = true
	export let sideBySideMinWidth = Breakpoints.lg

	let diffEditor: meditor.IStandaloneDiffEditor
	let diffDivEl: HTMLDivElement | null = null

	function loadDiffEditor() {
		diffEditor = meditor.createDiffEditor(diffDivEl!, {
			automaticLayout,
			renderSideBySide: window.innerWidth >= sideBySideMinWidth,
			domReadOnly: true,
			readOnly: true,
			minimap: {
				enabled: false
			},
			fixedOverflowWidgets,
			scrollBeyondLastLine: false,
			lineDecorationsWidth: 15,
			lineNumbersMinChars: 2,
			autoDetectHighContrast: true,
			scrollbar: { alwaysConsumeMouseWheel: false }
		})
	}

	export function setDiff(
		original: string,
		modified: string,
		lang: 'typescript' | 'python' | 'go' | 'shell' | 'sql' | 'graphql'
	): void {
		diffEditor.setModel({
			original: meditor.createModel(original, lang),
			modified: meditor.createModel(modified, lang)
		})

		if (lang !== 'shell') {
			diffEditor?.getModifiedEditor().getAction('editor.action.formatDocument')?.run()
		}
	}

	export function getOriginal(): string {
		return diffEditor.getModel()?.original.getValue() ?? ''
	}

	export function getModified(): string {
		return diffEditor.getModel()?.modified.getValue() ?? ''
	}

	export function show(): void {
		diffDivEl?.classList.remove('hidden')
	}
	export function hide(): void {
		diffDivEl?.classList.add('hidden')
	}

	function onResize() {
		diffEditor.updateOptions({ renderSideBySide: window.innerWidth >= sideBySideMinWidth })
	}

	onMount(() => {
		if (BROWSER) {
			loadDiffEditor()
			window.addEventListener('resize', onResize)
			return () => {
				diffEditor?.dispose()
				window.removeEventListener('resize', onResize)
			}
		}
	})
</script>

<div bind:this={diffDivEl} class="{$$props.class} editor" />
