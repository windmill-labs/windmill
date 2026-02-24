<script lang="ts">
	import { Group } from 'lucide-svelte'
	import { getNodeColorClasses } from '$lib/components/graph'
	import { NOTE_COLORS, NoteColor } from './noteColors'
	import { twMerge } from 'tailwind-merge'
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import GroupNoteArea from './GroupNoteArea.svelte'

	interface Props {
		summary?: string
		selected?: boolean
		stepCount?: number
		fullWidth?: boolean
		color?: string
		note?: string
		showNote?: boolean
		editMode?: boolean
		onSummaryUpdate?: (text: string) => void
		onNoteUpdate?: (text: string) => void
		onHeightChange?: (height: number) => void
	}

	let {
		summary,
		selected = false,
		stepCount,
		fullWidth = false,
		color,
		note,
		showNote = false,
		editMode = false,
		onSummaryUpdate,
		onNoteUpdate,
		onHeightChange
	}: Props = $props()

	let noteColorConfig = $derived(
		color ? (NOTE_COLORS[color as NoteColor] ?? NOTE_COLORS[NoteColor.BLUE]) : undefined
	)

	let defaultColorClasses = $derived(getNodeColorClasses(undefined, selected))

	// Inline summary editing
	let editingSummary = $state(false)
	let summaryInput = $state('')
	let summaryInputElement: HTMLInputElement | undefined = $state(undefined)

	function startEditingSummary() {
		if (!editMode) return
		editingSummary = true
		summaryInput = summary ?? ''
		requestAnimationFrame(() => {
			summaryInputElement?.focus()
			summaryInputElement?.select()
		})
	}

	function saveSummary() {
		editingSummary = false
		const trimmed = summaryInput.trim()
		if (trimmed !== (summary ?? '')) {
			onSummaryUpdate?.(trimmed)
		}
	}

	function handleSummaryKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			saveSummary()
		} else if (event.key === 'Escape') {
			editingSummary = false
		}
	}

	// Reset height to 0 when note is hidden
	$effect(() => {
		if (!showNote) {
			onHeightChange?.(0)
		}
	})
</script>

<div
	class={twMerge(
		'w-full module cursor-pointer max-w-full',
		fullWidth ? 'rounded-t-md' : 'rounded-md drop-shadow-base',
		noteColorConfig ? noteColorConfig.background : defaultColorClasses.bg,
		noteColorConfig ? noteColorConfig.text : ''
	)}
	style={fullWidth ? '' : 'width: 275px;'}
>
	<div
		class={twMerge(
			'absolute z-0 outline-offset-0 inset-0',
			fullWidth ? 'rounded-t-md' : 'rounded-md',
			noteColorConfig ? noteColorConfig.outline : defaultColorClasses.outline
		)}
	></div>
	<div class="flex items-center w-full gap-1.5 px-2 h-[34px] relative z-1">
		<Group size={14} />
		{#if editingSummary}
			<input
				bind:this={summaryInputElement}
				bind:value={summaryInput}
				class="text-2xs font-medium bg-transparent border-none p-0 m-0 outline-none min-w-0 flex-1 nodrag nowheel"
				placeholder="Group"
				onblur={saveSummary}
				onkeydown={handleSummaryKeydown}
				onclick={stopPropagation(preventDefault(() => {}))}
				onpointerdown={stopPropagation(() => {})}
				spellcheck="false"
			/>
		{:else}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<span
				class="text-2xs font-medium truncate {editMode ? 'cursor-text hover:opacity-80' : ''}"
				onclick={editMode ? stopPropagation(preventDefault(startEditingSummary)) : undefined}
				onpointerdown={editMode ? stopPropagation(preventDefault(() => {})) : undefined}
			>{summary || 'Group'}</span>
		{/if}
		{#if stepCount != null}
			<span class="text-2xs opacity-60">{stepCount} node{stepCount !== 1 ? 's' : ''}</span>
		{/if}
	</div>
	{#if showNote}
		<div class="relative z-1">
			<GroupNoteArea
				note={note ?? ''}
				{color}
				{editMode}
				onHeightChange={(h) => onHeightChange?.(h)}
				onNoteUpdate={(text) => onNoteUpdate?.(text)}
			/>
		</div>
	{/if}
</div>
