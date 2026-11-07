<script lang="ts" generics="A = undefined">
	import { onDestroy, untrack, type Snippet } from 'svelte'
	import {
		FileDiff,
		type DiffLineAnnotation,
		type FileContents,
		type FileDiffMetadata,
		type FileDiffOptions
	} from '@pierre/diffs'
	import type { WorkerPoolManager } from '@pierre/diffs/worker'
	import type { Editor, EditorOptions, Marker } from '@pierre/diffs/edit'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import { PIERRE_THEME, pierreLang } from './pierre'
	import { snippetElements } from './snippetElements'

	interface Props {
		/** Rendered from these two texts unless `fileDiff` is given. */
		before?: string
		after?: string
		fileDiff?: FileDiffMetadata
		filename?: string
		/** Monaco language id. */
		lang?: string
		/** Monaco language id of the old side, when it differs. */
		beforeLang?: string
		options?: Omit<FileDiffOptions<A, undefined>, 'renderAnnotation'>
		workerPool?: WorkerPoolManager
		annotations?: DiffLineAnnotation<A>[]
		annotation?: Snippet<[DiffLineAnnotation<A>]>
		headerMetadata?: Snippet<[FileDiffMetadata]>
		/** Attaches an editor to the new side; turning it off ends the session and calls `onEdited`. */
		editable?: boolean
		editorOptions?: EditorOptions<'file-diff', A, undefined>
		markers?: Marker[]
		onEdited?: (after: string, fileDiff: FileDiffMetadata) => void
		onEditChange?: (after: string) => void
		onEditor?: (editor: Editor<'file-diff', A, undefined> | undefined) => void
		class?: string
	}

	let {
		before = '',
		after = '',
		fileDiff,
		filename = 'file',
		lang,
		beforeLang,
		options,
		workerPool,
		annotations,
		annotation,
		headerMetadata,
		editable = false,
		editorOptions,
		markers,
		onEdited,
		onEditChange,
		onEditor,
		class: className = ''
	}: Props = $props()

	const isDark = useIsDarkMode()
	const elements = snippetElements()
	let container: HTMLDivElement | undefined = $state()
	let instance: FileDiff<A> | undefined = $state()
	let editor: Editor<'file-diff', A, undefined> | undefined = $state()

	// FileDiff compares inputs by reference, so a new object only when the text changes.
	const oldFile: FileContents = $derived({
		name: filename,
		contents: before,
		lang: pierreLang(beforeLang ?? lang)
	})
	const newFile: FileContents = $derived({
		name: filename,
		contents: after,
		lang: pierreLang(lang)
	})

	function buildOptions(): FileDiffOptions<A, undefined> {
		return {
			theme: PIERRE_THEME,
			...options,
			themeType: isDark.val ? 'dark' : 'light',
			renderAnnotation: annotation ? (a) => elements.render(annotation, a) : undefined,
			renderHeaderMetadata: headerMetadata
				? (d) => elements.render(headerMetadata, d, 'span')
				: undefined,
			onEditChange: (event) => onEditChange?.(event.file.contents),
			onEditComplete: (event) => {
				const text = event.newFile?.contents ?? ''
				onEdited?.(text, event.fileDiff)
				return 'accept'
			}
		}
	}

	$effect(() => {
		if (!container) return
		const fd = untrack(() => new FileDiff<A>(buildOptions(), workerPool))
		instance = fd
		return () => {
			fd.cleanUp()
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
		const input = fileDiff ? { fileDiff } : { oldFile, newFile }
		instance.render({ ...input, lineAnnotations: annotations, containerWrapper: container })
	})

	$effect(() => {
		if (!instance || !editable) return
		const fd = instance
		let dispose: (() => void) | undefined
		let cancelled = false
		// The edit entry point is its own chunk; only editable views pay for it.
		import('@pierre/diffs/edit').then(({ Editor }) => {
			if (cancelled) return
			const e = new Editor<'file-diff', A, undefined>(
				'file-diff',
				untrack(() => editorOptions)
			)
			dispose = e.edit(fd)
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
