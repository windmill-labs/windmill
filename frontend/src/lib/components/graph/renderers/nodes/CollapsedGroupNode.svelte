<script lang="ts">
	import { stopPropagation, preventDefault } from 'svelte/legacy'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { CollapsedGroupN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	import GroupNodeCard from '../../GroupNodeCard.svelte'
	import GroupActionBar from '../../GroupActionBar.svelte'
	import { getGroupEditorContext } from '../../groupEditor.svelte'
	import { NOTE_COLORS, NoteColor } from '../../noteColors'

	interface Props {
		data: CollapsedGroupN['data']
		id: string
	}

	let { data, id }: Props = $props()

	const { selectionManager } = getGraphContext()
	const groupEditorContext = getGroupEditorContext()

	let selected = $derived(!!(selectionManager && selectionManager.isNodeSelected(id)))
	let hover = $state(false)
	let noteHeight = $state(0)

	let group = $derived(
		groupEditorContext?.groupEditor.getGroups().find((g) => g.id === data.groupId)
	)

	let colorConfig = $derived(
		data.color
			? (NOTE_COLORS[data.color as NoteColor] ?? NOTE_COLORS[NoteColor.BLUE])
			: NOTE_COLORS[NoteColor.BLUE]
	)
	let borderColorClass = $derived(colorConfig.outline.replace(/outline-/g, 'border-'))
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="relative" onmouseenter={() => (hover = true)} onmouseleave={() => (hover = false)}>
			<!-- Step count badge — top-left -->
			<button
				class="absolute -top-5 font-normal left-0 text-3xs text-secondary opacity-60 hover:opacity-100 hover:text-blue-500 z-10"
				onclick={stopPropagation(
					preventDefault(() => data.eventHandlers.expandGroup(data.groupId))
				)}
				onpointerdown={stopPropagation(preventDefault(() => {}))}
			>
				{data.stepCount} step{data.stepCount !== 1 ? 's' : ''}
			</button>

			<!-- Stacked layers behind the card -->
			<div
				class="absolute inset-0 left-2 h-9 rounded-md border z-[-1] {colorConfig.background} {borderColorClass}"
				style="top: {3 + noteHeight}px; width: 259px;"
			></div>
			<div
				class="absolute inset-0 left-4 h-9 rounded-md border z-[-2] {colorConfig.background} {borderColorClass}"
				style="top: {7 + noteHeight}px; width: 243px;"
			></div>

			<GroupNodeCard
				summary={data.summary}
				color={data.color}
				{selected}
				note={data.note}
				showNote={data.showNotes && data.note != null}
				editMode={data.editMode}
				modules={data.modules}
				onSummaryUpdate={(text) =>
					groupEditorContext?.groupEditor.updateSummary(data.groupId, text)}
				onNoteUpdate={(text) => groupEditorContext?.groupEditor.updateNote(data.groupId, text)}
				onHeightChange={(h) => {
					noteHeight = h
					groupEditorContext?.groupEditor.setNoteHeight(data.groupId, h)
				}}
			/>

			{#if data.editMode}
				<GroupActionBar
					note={data.note}
					color={data.color}
					collapsedByDefault={group?.collapsed_by_default ?? false}
					collapsed={true}
					showAll={hover || selected}
					onAddNote={() => groupEditorContext?.groupEditor.addNote(data.groupId)}
					onRemoveNote={() => groupEditorContext?.groupEditor.removeNote(data.groupId)}
					onToggleCollapse={() => data.eventHandlers.expandGroup(data.groupId)}
					onUpdateColor={(c) => groupEditorContext?.groupEditor.updateColor(data.groupId, c)}
					onUpdateCollapsedDefault={(v) =>
						groupEditorContext?.groupEditor.updateCollapsedDefault(data.groupId, v)}
					onDeleteGroup={() => groupEditorContext?.groupEditor.deleteGroup(data.groupId)}
				/>
			{/if}
		</div>
	{/snippet}
</NodeWrapper>
