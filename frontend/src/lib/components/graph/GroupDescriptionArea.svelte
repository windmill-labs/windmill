<script lang="ts">
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import { NOTE_COLORS, NoteColor } from './noteColors'

	interface Props {
		description: string
		color?: string
		editMode: boolean
		onHeightChange: (height: number) => void
		onDescriptionUpdate: (text: string) => void
	}

	let { description, color, editMode, onHeightChange, onDescriptionUpdate }: Props =
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

	// Measure height and report to parent
	$effect(() => {
		if (containerElement) {
			const height = containerElement.clientHeight
			onHeightChange(height)
		}
	})

	// Also observe resize for dynamic content
	$effect(() => {
		if (!containerElement) return
		const observer = new ResizeObserver((entries) => {
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
		textContent = description
		requestAnimationFrame(() => {
			textareaElement?.focus()
		})
	}

	function handleSave() {
		editing = false
		if (textContent !== description) {
			onDescriptionUpdate(textContent)
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			editing = false
			textContent = description
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={containerElement}
	class="w-full border-t {borderColorClass} {noteColorConfig.background}"
	ondblclick={handleDoubleClick}
>
	{#if editing}
		<textarea
			bind:this={textareaElement}
			bind:value={textContent}
			class="w-full shadow-none resize-none text-2xs overflow-y-auto border-none bg-transparent p-2 nodrag nowheel focus:outline-none {noteColorConfig.text}"
			placeholder="Write a description (markdown supported)"
			onblur={handleSave}
			onkeydown={handleKeydown}
			spellcheck="false"
			rows="3"
		></textarea>
	{:else if description}
		<div class="w-full text-2xs break-words overflow-hidden p-2 {noteColorConfig.text}">
			<GfmMarkdown md={description} noPadding />
		</div>
	{:else}
		<div class="text-2xs italic opacity-60 p-2 {noteColorConfig.text}">
			Double click to add a description
		</div>
	{/if}
</div>
