<script lang="ts">
	import { NodeResizer } from '@xyflow/svelte'
	import { X, Lock, Unlock } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import { fade } from 'svelte/transition'
	import NoteColorPicker from '../../NoteColorPicker.svelte'
	import { NoteColor, NOTE_COLORS, DEFAULT_NOTE_COLOR } from '../../noteColors'
	import { Button } from '$lib/components/common'

	interface Props {
		data: {
			text: string
			color: NoteColor
			locked?: boolean
			isGroupNote?: boolean
			onUpdate?: (text: string) => void
			onDelete?: () => void
			onColorChange?: (color: NoteColor) => void
			onSizeChange?: (size: { width: number; height: number }) => void
			onLockToggle?: (locked: boolean) => void
		}
		selected?: boolean
		dragging?: boolean
	}

	let { data, selected = false, dragging = false }: Props = $props()

	let textareaElement: HTMLTextAreaElement | undefined = $state(undefined)
	let contentElement: HTMLDivElement | undefined = $state(undefined)
	let editMode = $state(false)
	let hovering = $state(false)
	let textContent = $state(data.text ?? '')
	let contentHeight = $state(0)

	function calculateContentHeight() {
		let calculatedHeight = 0
		let currentWidth = 300 // Default width

		if (editMode && textareaElement) {
			// For textarea in edit mode
			textareaElement.style.height = 'auto'
			const scrollHeight = textareaElement.scrollHeight
			const minHeight = 60 // Minimum height in pixels
			const maxHeight = 400 // Maximum height in pixels
			calculatedHeight = Math.max(minHeight, Math.min(maxHeight, scrollHeight))
			textareaElement.style.height = `${calculatedHeight}px`

			// Get current width from parent element
			const parentElement = textareaElement.closest('.svelte-flow__node')
			if (parentElement) {
				currentWidth = parentElement.getBoundingClientRect().width
			}
		} else if (!editMode && contentElement) {
			// For content in display mode
			const scrollHeight = contentElement.scrollHeight
			const minHeight = 60
			const maxHeight = 400
			calculatedHeight = Math.max(minHeight, Math.min(maxHeight, scrollHeight))

			// Get current width from parent element
			const parentElement = contentElement.closest('.svelte-flow__node')
			if (parentElement) {
				currentWidth = parentElement.getBoundingClientRect().width
			}
		}

		contentHeight = calculatedHeight

		// Update note size if handler is available
		if (data.onSizeChange && contentHeight > 0) {
			data.onSizeChange({ width: currentWidth, height: contentHeight + 40 }) // Add extra padding for note UI
		}
	}

	function handleTextSave() {
		// Only update parent when done editing
		data.onUpdate?.(textContent)
		// Recalculate height after saving
		setTimeout(calculateContentHeight, 0)
	}

	function handleDelete(event?: Event) {
		event?.preventDefault?.()
		event?.stopPropagation?.()
		// Call the delete callback
		data.onDelete?.()
	}

	function handleColorChange(color: NoteColor) {
		data.onColorChange?.(color)
	}

	function handleLockToggle(event?: Event) {
		event?.preventDefault?.()
		event?.stopPropagation?.()
		data.onLockToggle?.(!data.locked)
	}

	// Get color configuration for current color
	const colorConfig = $derived(NOTE_COLORS[data.color] || NOTE_COLORS[DEFAULT_NOTE_COLOR])

	function handleDoubleClick(event: Event) {
		console.log('Double click detected', { editMode, selected, dragging })
		event.preventDefault()
		event.stopPropagation()

		// Don't allow editing if note is locked
		if (data.locked) {
			return
		}

		textContent = data.text ?? '' // Initialize local state with current data
		editMode = true
		// Focus the textarea after a short delay to ensure it's rendered
		setTimeout(() => {
			textareaElement?.focus()
			calculateContentHeight()
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
		if (!selected && editMode) {
			handleTextSave() // Save changes before exiting
			editMode = false
		}
	})

	// Auto-resize when content changes
	$effect(() => {
		textContent // Track textContent changes
		if (editMode || contentElement) {
			setTimeout(calculateContentHeight, 0)
		}
	})

	// Calculate initial content height
	$effect(() => {
		if (data.text && !editMode && contentElement) {
			setTimeout(calculateContentHeight, 0)
		}
	})

	let colorPickerIsOpen = $state(false)
</script>

<div
	class={twMerge(
		'relative w-full h-full rounded-md group hover:outline outline-1',
		colorConfig.background,
		colorConfig.text,
		colorConfig.outlineHover,
		selected ? 'outline' : '',
		selected ? colorConfig.outline : '',
		editMode ? 'outline-0' : ''
	)}
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
	<!-- Action buttons -->
	<div class="absolute -top-10 -right-2.5 p-2 w-32 h-12 group flex justify-end">
		<div
			class={twMerge(
				'hidden group-hover:flex flex-row gap-2 h-fit',
				hovering || editMode || colorPickerIsOpen || selected ? 'flex' : ''
			)}
		>
			<!-- Lock/Unlock button -->
			<Button
				variant="subtle"
				unifiedSize="sm"
				title={data.locked ? 'Unlock note' : 'Lock note'}
				aria-label={data.locked ? 'Unlock note' : 'Lock note'}
				startIcon={{ icon: data.locked ? Lock : Unlock }}
				onClick={handleLockToggle}
				iconOnly
			/>

			<!-- Color picker -->
			{#if !data.locked}
				<NoteColorPicker
					selectedColor={data.color}
					onColorChange={handleColorChange}
					bind:isOpen={colorPickerIsOpen}
				/>
			{/if}

			<!-- Delete button -->
			{#if !data.locked}
				<Button
					variant="subtle"
					unifiedSize="sm"
					title="Delete note"
					aria-label="Delete note"
					startIcon={{ icon: X }}
					onClick={handleDelete}
					iconOnly
					destructive
				/>
			{/if}
		</div>
	</div>

	<!-- Hover help text -->
	{#if hovering || selected}
		{#if !editMode && data.text}
			<div
				in:fade={{ duration: 200 }}
				class="absolute -top-5 h-5 left-0 text-2xs text-secondary rounded-md z-10 transition-opacity duration-300"
			>
				{data.locked ? 'Note is locked' : 'Double click to edit'}
			</div>
		{:else if !data.locked}
			<div
				in:fade={{ duration: 200 }}
				class="absolute -top-5 h-5 left-0 text-2xs text-secondary rounded-md z-10 transition-opacity duration-300"
				>GH Markdown</div
			>
		{/if}
	{/if}

	<!-- Note content -->
	<div class="w-full min-h-[60px] max-h-[400px] rounded-md">
		{#if editMode}
			<!-- Edit mode: show textarea -->
			<textarea
				bind:this={textareaElement}
				bind:value={textContent}
				class={twMerge(
					'windmillapp w-full min-h-[60px] max-h-[400px] shadow-none resize-none text-xs overflow-y-auto border-none rounded-md bg-transparent transition-colors p-4',
					colorConfig.text
				)}
				placeholder="Add your note here... (Markdown supported)"
				onblur={handleTextSave}
				oninput={calculateContentHeight}
				spellcheck="false"
				style="height: auto;"
			></textarea>
		{:else}
			<!-- Render mode: show markdown or empty state -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				bind:this={contentElement}
				class={twMerge(
					'w-full min-h-[60px] max-h-[400px] overflow-auto cursor-pointer flex items-start justify-start rounded-md p-4'
				)}
				ondblclick={handleDoubleClick}
			>
				{#if data.text}
					<div class={twMerge('w-full text-xs rounded-md', colorConfig.text)}>
						<GfmMarkdown md={data.text} noPadding />
					</div>
				{:else}
					<div class={twMerge('text-xs italic opacity-60', colorConfig.text)}>
						Double-click to add a note
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Node resizer - only visible when selected and not locked -->
	{#if !data.locked}
		<NodeResizer
			isVisible={selected && !dragging}
			minWidth={200}
			minHeight={60}
			lineClass="!border-4 !border-transparent !rounded-md"
			handleClass="!bg-transparent !w-4 !h-4 !border-none !rounded-md"
			onResizeEnd={(_, params) => {
				// Update note size when resizing ends
				if (data.onSizeChange && params.width !== undefined && params.height !== undefined) {
					data.onSizeChange({ width: params.width, height: params.height })
				}
			}}
		/>
	{/if}
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
