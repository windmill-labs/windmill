<script lang="ts">
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import { NOTE_COLORS, NoteColor } from './noteColors'
	import { stopPropagation, preventDefault } from 'svelte/legacy'

	interface Props {
		note: string
		color?: string
		collapsed?: boolean
		editMode: boolean
		onHeightChange: (height: number) => void
		onNoteUpdate: (text: string) => void
	}

	let { note, color, collapsed = false, editMode, onHeightChange, onNoteUpdate }: Props = $props()

	let editing = $state(false)
	let editHeight = $state(0)
	let textContent = $state('')
	let textareaElement: HTMLTextAreaElement | undefined = $state(undefined)
	let containerElement: HTMLDivElement | undefined = $state(undefined)

	function autoResize(el: HTMLTextAreaElement) {
		el.style.height = 'auto'
		el.style.height = el.scrollHeight + 'px'
	}

	let noteColorConfig = $derived(
		color
			? (NOTE_COLORS[color as NoteColor] ?? NOTE_COLORS[NoteColor.BLUE])
			: NOTE_COLORS[NoteColor.BLUE]
	)

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
		return () => {
			observer.disconnect()
			onHeightChange(0)
		}
	})

	function handleDoubleClick() {
		if (!editMode) return
		editHeight = containerElement?.clientHeight ?? 0
		editing = true
		textContent = note
		requestAnimationFrame(() => {
			if (textareaElement) {
				autoResize(textareaElement)
				textareaElement.focus()
			}
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
	class="nodrag nopan {collapsed ? 'mx-px' : 'w-full rounded-b-md'}"
>
	<div class={collapsed ? '' : 'w-full rounded-b-md'}>
		{#if editing}
			<textarea
				bind:this={textareaElement}
				bind:value={textContent}
				class="w-full shadow-none resize-none !text-2xs overflow-y-auto border-none bg-transparent p-1 nodrag nopan nowheel focus:outline-none select-text {noteColorConfig.text}"
				style:max-height="max({editHeight}px, 4lh)"
				oninput={() => textareaElement && autoResize(textareaElement)}
				placeholder="Write a note (markdown supported)"
				onblur={handleSave}
				onkeydown={handleKeydown}
				onpointerdown={stopPropagation(() => {})}
				spellcheck="false"
			></textarea>
		{:else if note}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="w-full text-2xs break-words overflow-hidden p-2 select-text {noteColorConfig.text} {editMode
					? 'cursor-pointer'
					: ''}"
				ondblclick={editMode ? stopPropagation(preventDefault(handleDoubleClick)) : undefined}
				onpointerdown={editMode ? stopPropagation(() => {}) : undefined}
			>
				<GfmMarkdown md={note} noPadding />
			</div>
		{:else}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="text-2xs italic opacity-60 p-2 {noteColorConfig.text} {editMode
					? 'cursor-pointer'
					: ''}"
				ondblclick={editMode ? stopPropagation(preventDefault(handleDoubleClick)) : undefined}
				onpointerdown={editMode ? stopPropagation(() => {}) : undefined}
			>
				Double click to edit the note
			</div>
		{/if}
	</div>
</div>
