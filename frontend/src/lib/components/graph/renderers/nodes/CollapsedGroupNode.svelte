<script lang="ts">
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
			<!-- Stacked layers behind the card -->
			<div
				class="absolute inset-0 top-[4px] left-2 h-9 rounded-md border z-[-1] {colorConfig.background} {borderColorClass}"
				style="width: 259px;"
			></div>
			<div
				class="absolute inset-0 top-[7px] left-4 h-9 rounded-md border z-[-2] {colorConfig.background} {borderColorClass}"
				style="width: 243px;"
			></div>

			<GroupNodeCard
				summary={data.summary}
				color={data.color}
				{selected}
				stepCount={data.stepCount}
				note={data.note}
				showNote={data.showNotes && data.note != null}
				editMode={data.editMode}
				modules={data.modules}
				onExpand={() => data.eventHandlers.expandGroup(data.groupId)}
				onSummaryUpdate={(text) =>
					groupEditorContext?.groupEditor.updateSummary(data.groupId, text)}
				onNoteUpdate={(text) => groupEditorContext?.groupEditor.updateNote(data.groupId, text)}
				onHeightChange={(h) => groupEditorContext?.groupEditor.setNoteHeight(data.groupId, h)}
			/>

			{#if data.editMode && (hover || selected)}
				<GroupActionBar
					note={data.note}
					color={data.color}
					collapsedByDefault={group?.collapsed_by_default ?? false}
					collapsed={true}
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
