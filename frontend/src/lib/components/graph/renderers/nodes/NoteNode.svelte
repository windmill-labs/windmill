<script lang="ts">
	import { NodeResizer } from '@xyflow/svelte'
	import { X, Lock, LockOpen } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import { fade } from 'svelte/transition'
	import NoteColorPicker from '../../NoteColorPicker.svelte'
	import { NoteColor, NOTE_COLORS, DEFAULT_NOTE_COLOR } from '../../noteColors'
	import { Button } from '$lib/components/common'
	import { getNoteEditorContext } from '../../noteEditor.svelte'

	interface Props {
		data: {
			noteId: string
			text: string
			color: NoteColor
			locked?: boolean
			isGroupNote?: boolean
			editMode?: boolean
			// Callback for layout calculations (needed in both edit and view modes)
			onTextHeightChange?: (height: number) => void
		}
		selected?: boolean
		dragging?: boolean
	}

	let { data, selected = false, dragging = false }: Props = $props()

	// Get NoteEditor context for edit mode
	const noteEditorContext = getNoteEditorContext()
	const isEditModeAvailable = $derived(!!noteEditorContext?.noteEditor && data.editMode)

	let textareaElement: HTMLTextAreaElement | undefined = $state(undefined)
	let editMode = $state(false)
	let hovering = $state(false)
	let textContent = $state(data.text ?? '')
	let containerHeight = $state(0)

	// Use data props directly - they're kept in sync by NoteManager observer
	const color = $derived(data.color ?? DEFAULT_NOTE_COLOR)
	const locked = $derived(data.locked ?? false)
	const textForDisplay = $derived(data.text ?? '')

	function handleTextSave() {
		// Only update when done editing
		if (isEditModeAvailable && noteEditorContext?.noteEditor) {
			noteEditorContext.noteEditor.updateText(data.noteId, textContent)
		}
	}

	function handleDelete(event?: Event) {
		event?.preventDefault?.()
		event?.stopPropagation?.()
		if (isEditModeAvailable && noteEditorContext?.noteEditor) {
			noteEditorContext.noteEditor.deleteNote(data.noteId)
		}
	}

	function handleColorChange(color: NoteColor) {
		if (isEditModeAvailable && noteEditorContext?.noteEditor) {
			noteEditorContext.noteEditor.updateColor(data.noteId, color)
		}
	}

	function handleLockToggle(event?: Event) {
		event?.preventDefault?.()
		event?.stopPropagation?.()
		if (isEditModeAvailable && noteEditorContext?.noteEditor) {
			noteEditorContext.noteEditor.updateLock(data.noteId, !locked)
		}
	}

	// Get color configuration for current color
	const colorConfig = $derived(NOTE_COLORS[color])

	function handleDoubleClick(event: Event) {
		event.preventDefault()
		event.stopPropagation()

		// Don't allow editing if note is locked or edit mode is not available
		if (locked || !isEditModeAvailable) {
			return
		}

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
		if (!selected && editMode) {
			handleTextSave() // Save changes before exiting
			editMode = false
		}
	})

	// Track content height and notify parent
	let previousContainerHeight = $state(0)
	$effect(() => {
		if (containerHeight > 0 && containerHeight !== previousContainerHeight) {
			previousContainerHeight = containerHeight
			data.onTextHeightChange?.(containerHeight)
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
	<!-- Action buttons - only show in edit mode -->
	{#if isEditModeAvailable}
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
					title={locked ? 'Unlock note' : 'Lock note'}
					aria-label={locked ? 'Unlock note' : 'Lock note'}
					startIcon={{ icon: locked ? Lock : LockOpen }}
					onClick={handleLockToggle}
					iconOnly
				/>

				<!-- Color picker -->
				{#if !locked}
					<NoteColorPicker
						selectedColor={color}
						onColorChange={handleColorChange}
						bind:isOpen={colorPickerIsOpen}
					/>
				{/if}

				<!-- Delete button -->
				{#if !locked}
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
	{/if}

	<!-- Hover help text -->
	{#if hovering || selected}
		{#if !editMode && isEditModeAvailable}
			<div
				in:fade={{ duration: 200 }}
				class="absolute -top-5 h-5 left-0 text-2xs text-secondary rounded-md z-10 transition-opacity duration-300"
			>
				{locked
					? 'Note is locked'
					: isEditModeAvailable
						? 'Double click to edit'
						: 'View only mode'}
			</div>
		{:else if !locked && isEditModeAvailable}
			<div
				in:fade={{ duration: 200 }}
				class="absolute -top-5 h-5 left-0 text-2xs text-secondary rounded-md z-10 transition-opacity duration-300"
				>GH Markdown</div
			>
		{/if}
	{/if}

	<!-- Note content -->
	<div
		class={twMerge(
			'w-full min-h-[60px] max-h-[400px] rounded-md ',
			data.isGroupNote ? '' : 'h-full'
		)}
	>
		{#if editMode}
			<!-- Edit mode: show textarea -->
			<textarea
				bind:this={textareaElement}
				bind:value={textContent}
				class={twMerge(
					'windmillapp w-full shadow-none resize-none text-xs overflow-y-auto border-none rounded-md bg-transparent transition-colors p-4',
					colorConfig.text
				)}
				placeholder="Add your note here... (Markdown supported)"
				onblur={handleTextSave}
				spellcheck="false"
				style:height={data.isGroupNote ? `${containerHeight > 0 ? containerHeight : 60}px` : '100%'}
			></textarea>
		{:else}
			<!-- Render mode: show markdown or empty state -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class={twMerge(
					'w-full h-fit overflow-auto cursor-pointer flex items-start justify-start rounded-md p-4'
				)}
				ondblclick={handleDoubleClick}
				bind:clientHeight={containerHeight}
			>
				{#if textForDisplay}
					<div
						class={twMerge(
							'w-full text-xs rounded-md break-words overflow-hidden',
							colorConfig.text
						)}
					>
						<GfmMarkdown md={textForDisplay} noPadding />
					</div>
				{:else}
					<div class={twMerge('text-xs italic opacity-60', colorConfig.text)}>
						Double-click to add a note
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Node resizer - only visible when selected and not locked and edit mode is available -->
	{#if !locked && isEditModeAvailable}
		<NodeResizer
			isVisible={selected && !dragging}
			minWidth={200}
			minHeight={60}
			lineClass="!border-4 !border-transparent !rounded-md"
			handleClass="!bg-transparent !w-4 !h-4 !border-none !rounded-md"
			onResizeEnd={(_, params) => {
				// Update note size when resizing ends
				if (params.width !== undefined && params.height !== undefined) {
					const size = { width: params.width, height: params.height }
					if (isEditModeAvailable && noteEditorContext?.noteEditor) {
						// Use NoteEditor context in edit mode
						noteEditorContext.noteEditor.updateSize(data.noteId, size)
					}
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
