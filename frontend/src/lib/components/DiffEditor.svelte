<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { onMount } from 'svelte'

	import '@codingame/monaco-vscode-standalone-languages'
	import '@codingame/monaco-vscode-standalone-json-language-features'
	import '@codingame/monaco-vscode-standalone-typescript-language-features'
	import { editor as meditor, KeyMod, KeyCode } from 'monaco-editor'

	import { initializeVscode } from './vscode'
	import EditorTheme from './EditorTheme.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { ButtonType } from './common'

	const SIDE_BY_SIDE_MIN_WIDTH = 700

	export interface ButtonProp {
		text: string
		color?: ButtonType.Color
		onClick: () => void
	}

	interface Props {
		open?: boolean
		className?: string
		automaticLayout?: boolean
		fixedOverflowWidgets?: boolean
		defaultLang?: string
		defaultModifiedLang?: string
		defaultOriginal?: string
		defaultModified?: string
		readOnly?: boolean
		buttons?: ButtonProp[]
		modifiedModel?: meditor.ITextModel | meditor.IEditorModel
		inlineDiff?: boolean
	}

	let {
		open = false,
		className = '',
		automaticLayout = true,
		fixedOverflowWidgets = true,
		defaultLang,
		defaultModifiedLang,
		defaultOriginal = undefined,
		defaultModified = undefined,
		readOnly = false,
		buttons = [],
		modifiedModel,
		inlineDiff = false
	}: Props = $props()

	let diffEditor: meditor.IStandaloneDiffEditor | undefined = $state(undefined)
	let diffDivEl: HTMLDivElement | null = $state(null)
	let editorWidth: number = $state(SIDE_BY_SIDE_MIN_WIDTH)

	async function loadDiffEditor() {
		await initializeVscode()

		if (!diffDivEl) {
			return
		}

		diffEditor = meditor.createDiffEditor(diffDivEl!, {
			automaticLayout,
			renderSideBySide: inlineDiff ? false : editorWidth >= SIDE_BY_SIDE_MIN_WIDTH,
			originalEditable: false,
			readOnly,
			minimap: {
				enabled: false
			},
			fixedOverflowWidgets,
			scrollBeyondLastLine: false,
			lineDecorationsWidth: 15,
			lineNumbersMinChars: 2,
			scrollbar: { alwaysConsumeMouseWheel: false }
		})

		// In VSCode webview (iframe), clipboard operations need special handling
		// because the webview has restricted clipboard API access
		if (window.parent !== window) {
			const modifiedEditor = diffEditor.getModifiedEditor()
			modifiedEditor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyC, function () {
				document.execCommand('copy')
			})
			modifiedEditor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyX, function () {
				document.execCommand('cut')
			})
			modifiedEditor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyV, async function () {
				try {
					const text = await navigator.clipboard.readText()
					if (text) {
						const selection = modifiedEditor.getSelection()
						if (selection) {
							modifiedEditor.executeEdits('paste', [
								{
									range: selection,
									text: text,
									forceMoveMarkers: true
								}
							])
						}
					}
				} catch (e) {
					document.execCommand('paste')
				}
			})
		}

		if (defaultLang !== undefined) {
			setupModel(defaultLang, defaultOriginal, defaultModified, defaultModifiedLang)
		}
	}

	export function setupModel(
		lang: string,
		original?: string,
		modified?: string,
		modifiedLang?: string
	) {
		const o = meditor.createModel(original ?? '', lang)
		const m = modifiedModel ?? meditor.createModel(modified ?? '', modifiedLang ?? lang)
		diffEditor?.setModel({
			original: o,
			modified: m as meditor.ITextModel
		})
	}

	export function setOriginal(code: string) {
		diffEditor?.getModel()?.original?.setValue(code)
		defaultOriginal = code
	}

	export function getOriginal(): string {
		return diffEditor?.getModel()?.original.getValue() ?? ''
	}

	export function setModified(code: string) {
		diffEditor?.getModel()?.modified?.setValue(code)
		defaultModified = code
	}

	export function setModifiedModel(model: meditor.ITextModel) {
		const curr = diffEditor?.getModel()
		if (!curr) return
		diffEditor?.setModel({
			original: curr.original,
			modified: model
		})
	}

	export function showWithModelAndOriginal(
		original: string,
		model: meditor.ITextModel | meditor.IEditorModel
	) {
		setOriginal(original)
		setModifiedModel(model as meditor.ITextModel)
		show()
	}
	export function getModified(): string {
		return diffEditor?.getModel()?.modified.getValue() ?? ''
	}

	export function show(): void {
		open = true
	}
	export function hide(): void {
		open = false
	}

	$effect(() => {
		if (open && diffDivEl) {
			loadDiffEditor()
		}
	})

	$effect(() => {
		if (diffEditor) {
			diffEditor.updateOptions({
				renderSideBySide: inlineDiff ? false : editorWidth >= SIDE_BY_SIDE_MIN_WIDTH
			})
		}
	})

	onMount(() => {
		if (BROWSER) {
			return () => {
				diffEditor?.dispose()
			}
		}
	})
</script>

{#if open}
	<EditorTheme />
	<div
		bind:this={diffDivEl}
		class={twMerge('editor nonmain-editor', className)}
		bind:clientWidth={editorWidth}
	></div>
	{#if buttons.length > 0}
		<div
			class="absolute flex flex-row gap-2 bottom-10 left-1/2 z-10 -translate-x-1/2 rounded-md p-1 w-full justify-center"
		>
			{#each buttons as button}
				<Button on:click={button.onClick} variant="contained" size="sm" color={button.color}
					>{button.text}</Button
				>
			{/each}
		</div>
	{/if}
{/if}
