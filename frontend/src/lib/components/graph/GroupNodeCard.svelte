<script lang="ts">
	import { Group } from 'lucide-svelte'
	import { getNodeColorClasses } from '$lib/components/graph'
	import { NOTE_COLORS, NoteColor } from './noteColors'
	import { twMerge } from 'tailwind-merge'
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import GroupNoteArea from './GroupNoteArea.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import GroupModuleIcons from './GroupModuleIcons.svelte'
	import type { FlowModule } from '$lib/gen'

	interface Props {
		summary?: string
		selected?: boolean
		stepCount?: number
		color?: string
		note?: string
		showNote?: boolean
		editMode?: boolean
		modules?: FlowModule[]
		onExpand?: () => void
		onSummaryUpdate?: (text: string) => void
		onNoteUpdate?: (text: string) => void
		onHeightChange?: (height: number) => void
	}

	let {
		summary,
		selected = false,
		stepCount,
		color,
		note,
		showNote = false,
		editMode = false,
		modules,
		onExpand,
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
	let textInputComponent: TextInput | undefined = $state(undefined)

	function startEditingSummary() {
		if (!editMode) return
		editingSummary = true
		summaryInput = summary ?? ''
		requestAnimationFrame(() => {
			textInputComponent?.focus()
			textInputComponent?.select()
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
		'shadow-sm rounded-md overflow-clip',
		'bg-surface-tertiary'
	)}
	style="width: 275px;"
>
	<div
		class={twMerge(
			'absolute z-0 outline-offset-0 inset-0',
			'rounded-md',
			noteColorConfig ? noteColorConfig.outline : defaultColorClasses.outline
		)}
	></div>
	<div class="flex items-center w-full gap-1.5 px-2 h-[34px] relative z-1">
		{#if modules && modules.length > 0}
			<GroupModuleIcons {modules} />
		{:else}
			<Group size={14} />
		{/if}
		<div class="absolute inset-x-0 flex items-center justify-center h-[34px] pointer-events-none px-8">
			{#if editingSummary}
				<TextInput
					bind:this={textInputComponent}
					bind:value={summaryInput}
					size="xs"
					class="!bg-transparent !border-transparent !shadow-none !text-2xs !font-medium !p-0 !m-0 !min-w-0 w-full text-center !min-h-0 !h-auto nodrag nowheel pointer-events-auto"
					inputProps={{
						placeholder: 'Group',
						onblur: saveSummary,
						onkeydown: handleSummaryKeydown,
						spellcheck: false
					}}
				/>
			{:else}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<span
					class="text-2xs font-medium truncate text-center pointer-events-auto {editMode
						? 'cursor-text rounded px-0.5 -mx-0.5 hover:bg-black/10 dark:hover:bg-white/10'
						: ''}"
					onclick={editMode ? stopPropagation(preventDefault(startEditingSummary)) : undefined}
					onpointerdown={editMode ? stopPropagation(preventDefault(() => {})) : undefined}
					>{summary || 'Group'}</span
				>
			{/if}
		</div>
		<div class="flex-1"></div>
		{#if stepCount != null}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<span
				class="text-3xs opacity-60 shrink-0 whitespace-nowrap {noteColorConfig
					? noteColorConfig.text
					: ''} {onExpand
					? 'cursor-pointer hover:opacity-100 hover:text-blue-500 dark:hover:text-blue-400'
					: ''}"
				onclick={onExpand ? stopPropagation(preventDefault(onExpand)) : undefined}
				>{stepCount} node{stepCount !== 1 ? 's' : ''}</span
			>
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
