<script lang="ts">
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import { NOTE_COLORS, NoteColor } from './noteColors'
	import { stopPropagation, preventDefault } from 'svelte/legacy'

	interface Props {
		note: string
		color?: string
		editMode: boolean
		onHeightChange: (height: number) => void
		onNoteUpdate: (text: string) => void
	}

	let { note, color, editMode, onHeightChange, onNoteUpdate }: Props =
		$props()

	let editing = $state(false)
	let textContent = $state('')
	let textareaElement: HTMLTextAreaElement | undefined = $state(undefined)
	let containerElement: HTMLDivElement | undefined = $state(undefined)

	let noteColorConfig = $derived(
		color ? (NOTE_COLORS[color as NoteColor] ?? NOTE_COLORS[NoteColor.BLUE]) : NOTE_COLORS[NoteColor.BLUE]
	)

	// Derive border class from outline (e.g. "outline-yellow-300" → "border-yellow-300")
	let borderColorClass = $derived(noteColorConfig.outline.replace(/outline-/g, 'border-'))

	// Measure height and report to parent (skip while editing to avoid full graph rebuilds)
	$effect(() => {
		if (containerElement && !editing) {
			const height = containerElement.clientHeight
			onHeightChange(height)
		}
	})

	// Also observe resize for dynamic content
	$effect(() => {
		if (!containerElement) return
		const observer = new ResizeObserver((entries) => {
			if (editing) return
			for (const entry of entries) {
				onHeightChange(entry.contentRect.height)
			}
		})
		observer.observe(containerElement)
		return () => observer.disconnect()
	})

	function handleDoubleClick() {
		if (!editMode) return
		editing = true
		textContent = note
		requestAnimationFrame(() => {
			textareaElement?.focus()
		})
	}

	function handleSave() {
		editing = false
		if (textContent !== note) {
			onNoteUpdate(textContent)
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		event.stopPropagation()
		if (event.key === 'Escape') {
			editing = false
			textContent = note
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={containerElement}
	class="w-full border-t nodrag nopan nowheel {borderColorClass} {noteColorConfig.background}"
>
	{#if editing}
		<textarea
			bind:this={textareaElement}
			bind:value={textContent}
			class="w-full shadow-none resize-none text-2xs overflow-y-auto border-none bg-transparent p-2 nodrag nopan nowheel focus:outline-none {noteColorConfig.text}"
			placeholder="Write a note (markdown supported)"
			onblur={handleSave}
			onkeydown={handleKeydown}
			onpointerdown={stopPropagation(() => {})}
			spellcheck="false"
			rows="3"
		></textarea>
	{:else if note}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-full text-2xs break-words overflow-hidden p-2 {noteColorConfig.text}"
			ondblclick={editMode ? stopPropagation(preventDefault(handleDoubleClick)) : undefined}
			onpointerdown={editMode ? stopPropagation(() => {}) : undefined}
		>
			<GfmMarkdown md={note} noPadding />
		</div>
	{:else}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="text-2xs italic opacity-60 p-2 {noteColorConfig.text}"
			ondblclick={editMode ? stopPropagation(preventDefault(handleDoubleClick)) : undefined}
			onpointerdown={editMode ? stopPropagation(() => {}) : undefined}
		>
			Double click to add a note
		</div>
	{/if}
</div>
