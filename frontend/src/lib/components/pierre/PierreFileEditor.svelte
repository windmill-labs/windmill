<script lang="ts">
	import { onDestroy, untrack, type Snippet } from 'svelte'
	import { File, type FileContents, type FileOptions } from '@pierre/diffs'
	import type { WorkerPoolManager } from '@pierre/diffs/worker'
	import type { Editor, EditorOptions, Marker } from '@pierre/diffs/edit'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import { PIERRE_THEME, pierreLang } from './pierre'
	import { snippetElements } from './snippetElements'

	type FileEditorOptions = EditorOptions<'file', undefined, undefined>
	// The package does not export this type from `@pierre/diffs/edit`.
	type SelectionActionContext = Parameters<
		NonNullable<FileEditorOptions['renderSelectionAction']>
	>[0]

	interface Props {
		/** Initial text. Changing it replaces the document, so never feed `onChange` back into it. */
		code: string
		filename?: string
		/** Monaco language id. */
		lang?: string
		options?: FileOptions<undefined, undefined>
		workerPool?: WorkerPoolManager
		editorOptions?: Omit<FileEditorOptions, 'renderSelectionAction'>
		selectionAction?: Snippet<[SelectionActionContext]>
		markers?: Marker[]
		/** A later editor with the same key resumes this one's draft and undo history. */
		editStateKey?: string
		onChange?: (code: string) => void
		onEditor?: (editor: Editor<'file', undefined, undefined> | undefined) => void
		class?: string
	}

	let {
		code,
		filename = 'file',
		lang,
		options,
		workerPool,
		editorOptions,
		selectionAction,
		markers,
		editStateKey,
		onChange,
		onEditor,
		class: className = ''
	}: Props = $props()

	const isDark = useIsDarkMode()
	const elements = snippetElements()
	let container: HTMLDivElement | undefined = $state()
	let instance: File | undefined = $state()
	let editor: Editor<'file', undefined, undefined> | undefined = $state()

	const file: FileContents = $derived({ name: filename, contents: code, lang: pierreLang(lang) })

	function buildOptions(): FileOptions<undefined, undefined> {
		return {
			theme: PIERRE_THEME,
			...options,
			themeType: isDark.val ? 'dark' : 'light',
			onEditChange: (event) => onChange?.(event.file.contents),
			onEditComplete: () => 'accept'
		}
	}

	$effect(() => {
		if (!container) return
		const f = untrack(() => new File(buildOptions(), workerPool))
		instance = f
		return () => {
			f.cleanUp()
			instance = undefined
		}
	})

	$effect(() => {
		const opts = buildOptions()
		untrack(() => {
			if (!instance) return
			instance.setOptions(opts)
			instance.rerender()
		})
	})

	$effect(() => {
		if (!instance || !container) return
		instance.render({ file, containerWrapper: container })
	})

	$effect(() => {
		if (!instance) return
		const f = instance
		let dispose: (() => void) | undefined
		let cancelled = false
		import('@pierre/diffs/edit').then(({ Editor }) => {
			if (cancelled) return
			const e = new Editor<'file', undefined, undefined>(
				'file',
				untrack(() => ({
					...editorOptions,
					enabledSelectionAction: selectionAction !== undefined,
					renderSelectionAction: selectionAction
						? (ctx) => elements.render(selectionAction, ctx)
						: undefined
				})),
				untrack(() => editStateKey)
			)
			dispose = e.edit(f)
			editor = e
			onEditor?.(e)
		})
		return () => {
			cancelled = true
			dispose?.()
			editor = undefined
			onEditor?.(undefined)
		}
	})

	$effect(() => {
		editor?.setMarkers(markers ?? [])
	})

	onDestroy(() => elements.destroy())
</script>

<div bind:this={container} class={className}></div>
