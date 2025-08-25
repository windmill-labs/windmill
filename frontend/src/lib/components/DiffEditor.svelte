<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { onMount } from 'svelte'

	import '@codingame/monaco-vscode-standalone-languages'
	import '@codingame/monaco-vscode-standalone-json-language-features'
	import '@codingame/monaco-vscode-standalone-typescript-language-features'
	import { editor as meditor } from 'monaco-editor'

	import { initializeVscode } from './vscode'
	import EditorTheme from './EditorTheme.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'

	const SIDE_BY_SIDE_MIN_WIDTH = 700

	export interface ButtonProp {
		text: string
		color?: 'red' | 'green' | 'blue' | 'yellow' | 'purple' | 'orange' | 'pink' | 'gray' | 'black'
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
		buttons = []
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
			renderSideBySide: editorWidth >= SIDE_BY_SIDE_MIN_WIDTH,
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
		if (
			defaultOriginal !== undefined &&
			defaultModified !== undefined &&
			defaultLang !== undefined
		) {
			setupModel(defaultLang, defaultOriginal, defaultModified, defaultModifiedLang)
		}
	}

	export function setupModel(
		lang: string,
		original?: string,
		modified?: string,
		modifiedLang?: string
	) {
		diffEditor?.setModel({
			original: meditor.createModel('', lang),
			modified: meditor.createModel('', modifiedLang ?? lang)
		})
		if (original) {
			setOriginal(original)
		}
		if (modified) {
			setModified(modified)
		}
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

	export function getModified(): string {
		return diffEditor?.getModel()?.modified.getValue() ?? ''
	}

	export function show(): void {
		open = true
	}
	export function hide(): void {
		open = false
	}

	function onWidthChange(editorWidth: number) {
		diffEditor?.updateOptions({ renderSideBySide: editorWidth >= SIDE_BY_SIDE_MIN_WIDTH })
	}

	$effect(() => {
		if (open && diffDivEl) {
			loadDiffEditor()
		}
	})

	$effect(() => {
		onWidthChange(editorWidth)
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
