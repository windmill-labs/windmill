<script lang="ts">
	import { Group } from 'lucide-svelte'
	import { getNodeColorClasses } from '$lib/components/graph'
	import { NOTE_COLORS, NoteColor } from './noteColors'
	import { twMerge } from 'tailwind-merge'
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
		onNoteUpdate,
		onHeightChange
	}: Props = $props()

	let noteColorConfig = $derived(
		color ? (NOTE_COLORS[color as NoteColor] ?? NOTE_COLORS[NoteColor.BLUE]) : undefined
	)

	let defaultColorClasses = $derived(getNodeColorClasses(undefined, selected))

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
		<span class="text-2xs font-medium truncate">{summary || 'Group'}</span>
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
