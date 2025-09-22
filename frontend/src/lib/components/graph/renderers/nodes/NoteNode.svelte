<script lang="ts">
	import { NodeResizer } from '@xyflow/svelte'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import { fade } from 'svelte/transition'

	interface Props {
		data: {
			text: string
			color: string
			onUpdate?: (text: string) => void
			onDelete?: () => void
			onSizeChange?: (size: { width: number; height: number }) => void
		}
		selected?: boolean
		dragging?: boolean
	}

	let { data, selected = false, dragging = false }: Props = $props()

	let textareaElement: HTMLTextAreaElement | undefined = $state(undefined)
	let editMode = $state(false)
	let hovering = $state(false)

	function handleTextChange(event: Event) {
		const target = event.target as HTMLTextAreaElement
		console.log('Text change detected', target.value)
		// Call the update callback
		data.onUpdate?.(target.value)
	}

	function handleDelete(event: Event) {
		event.preventDefault()
		event.stopPropagation()
		// Call the delete callback
		data.onDelete?.()
	}

	function handleDoubleClick(event: Event) {
		console.log('Double click detected', { editMode, selected, dragging })
		event.preventDefault()
		event.stopPropagation()
		editMode = true
		// Focus the textarea after a short delay to ensure it's rendered
		setTimeout(() => {
			textareaElement?.focus()
		}, 0)
	}

	function handleMouseEnter() {
		hovering = true
	}

	function handleMouseLeave() {
		hovering = false
	}

	// Exit edit mode when note is deselected
	$effect(() => {
		if (!selected) {
			editMode = false
		}
	})
</script>

<div
	class={twMerge(
		'relative w-full h-full rounded-md border group',
		selected ? `outline outline-1 outline-lime-300` : ''
	)}
	style:background-color={data.color}
	onpointerup={() => {
		dragging = false
	}}
	ondragstart={() => {
		dragging = true
	}}
	ondragend={() => {
		dragging = false
	}}
	onmouseenter={handleMouseEnter}
	onmouseleave={handleMouseLeave}
	role="note"
>
	<!-- Delete button -->
	<button
		class="opacity-0 group-hover:opacity-100 hover:opacity-100 absolute -top-6 right-0 w-5 h-5 bg-transparent text-secondary hover:bg-red-500 hover:border-red-500 hover:text-white border rounded-md flex items-center justify-center transition-all duration-100 z-10"
		onclick={handleDelete}
		title="Delete note"
		aria-label="Delete note"
	>
		<X size="12" />
	</button>

	<!-- Hover help text -->
	{#if hovering}
		{#if !editMode && data.text}
			<div
				in:fade={{ duration: 200 }}
				class="absolute -top-5 h-5 left-0 text-xs text-gray-400 rounded-md z-10 transition-opacity duration-300"
			>
				Double click to edit
			</div>
		{:else}
			<div
				in:fade={{ duration: 200 }}
				class="absolute -top-5 h-5 left-0 text-xs text-gray-400 rounded-md z-10 transition-opacity duration-300"
				>GH Markdown</div
			>
		{/if}
	{/if}

	<!-- Note content -->
	<div class="p-2 h-full rounded-md">
		{#if editMode}
			<!-- Edit mode: show textarea -->
			<textarea
				bind:this={textareaElement}
				class="windmillapp w-full h-full min-h-0 shadow-none resize-none text-xs overflow-y-auto border-none rounded-md bg-transparent transition-colors p-2"
				placeholder="Add your note here... (Markdown supported)"
				value={data.text ?? ''}
				oninput={handleTextChange}
				spellcheck="false"
			></textarea>
		{:else}
			<!-- Render mode: show markdown or empty state -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="w-full h-full overflow-auto cursor-pointer flex items-start justify-center hover:bg-gray-400/10 rounded-md p-2"
				ondblclick={handleDoubleClick}
			>
				{#if data.text}
					<div class="w-full h-full text-xs rounded-md">
						<GfmMarkdown md={data.text} noPadding />
					</div>
				{:else}
					<div class="text-secondary text-xs italic"> Double-click to add a note </div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Node resizer - only visible when selected -->
	<NodeResizer
		isVisible={selected && !dragging}
		minWidth={200}
		minHeight={100}
		lineClass="!border-4 !border-transparent !rounded-md"
		handleClass="!bg-transparent !w-4 !h-4 !border-none !rounded-md"
		onResizeEnd={(_, params) => {
			// Update note size when resizing ends
			if (data.onSizeChange && params.width !== undefined && params.height !== undefined) {
				data.onSizeChange({ width: params.width, height: params.height })
			}
		}}
	/>
</div>

<style>
	textarea::placeholder {
		color: #6b7280;
		opacity: 0.7;
	}

	/* Remove default textarea styling */
	textarea {
		font-family: inherit;
		line-height: 1.4;
	}
</style>
