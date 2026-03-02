<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { CollapsedGroupN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	import GroupNodeCard from '../../GroupNodeCard.svelte'
	import GroupActionBar from '../../GroupActionBar.svelte'
	import { getGroupEditorContext } from '../../groupEditor.svelte'
	import StepCountTab from '../../StepCountTab.svelte'

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

</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="relative" onmouseenter={() => (hover = true)} onmouseleave={() => (hover = false)}>
			<StepCountTab stepCount={data.stepCount} color={data.color} onExpand={() => data.eventHandlers.expandGroup(data.groupId)} />


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
