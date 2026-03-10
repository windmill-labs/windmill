<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { CollapsedGroupN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	import GroupHeader from '../../GroupHeader.svelte'
	import GroupActionBar from '../../GroupActionBar.svelte'
	import GroupModuleIcons from '../../GroupModuleIcons.svelte'
	import GroupNoteArea from '../../GroupNoteArea.svelte'
	import { getGroupEditorContext } from '../../groupEditor.svelte'
	import { NOTE_COLORS, NoteColor } from '../../noteColors'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		data: CollapsedGroupN['data']
		id: string
	}

	let { data, id }: Props = $props()

	const { selectionManager } = getGraphContext()
	const groupEditorContext = getGroupEditorContext()

	let selected = $derived(!!(selectionManager && selectionManager.isNodeSelected(id)))

	let group = $derived(
		groupEditorContext?.groupEditor.getGroups().find((g) => g.id === data.groupId)
	)

	let noteColorConfig = $derived(
		NOTE_COLORS[(data.color as NoteColor) ?? NoteColor.BLUE] ?? NOTE_COLORS[NoteColor.BLUE]
	)

	let hover = $state(false)
	let menuOpen = $state(false)
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="relative" onmouseenter={() => (hover = true)} onmouseleave={() => (hover = false)}>
			<div
				class={twMerge(
					'w-full module cursor-pointer max-w-full',
					'rounded-md overflow-clip outline outline-1 -outline-offset-1',
					noteColorConfig.outlineHover,
					noteColorConfig.backgroundLight
				)}
				style="width: 275px;"
			>
				<div class="relative z-1">
					<GroupHeader
						summary={data.summary}
						color={data.color}
						collapsed={true}
						editMode={data.editMode}
						onToggleCollapse={() => data.eventHandlers.expandGroup(data.groupId)}
						onSummaryUpdate={(text) =>
							groupEditorContext?.groupEditor.updateSummary(data.groupId, text)}
					/>
					{#if data.modules && data.modules.length > 0}
						<div class="flex items-center justify-center w-full gap-1.5 px-2 h-[34px]">
							<GroupModuleIcons modules={data.modules} />
						</div>
					{/if}
				</div>

				{#if data.showNotes && data.note != null}
					<div class="relative z-1">
						<GroupNoteArea
							note={data.note ?? ''}
							color={data.color}
							editMode={data.editMode}
							onHeightChange={(h) => {
								groupEditorContext?.groupEditor.setNoteHeight(data.groupId, h)
							}}
							onNoteUpdate={(text) =>
								groupEditorContext?.groupEditor.updateNote(data.groupId, text)}
						/>
					</div>
				{/if}
			</div>

			{#if data.editMode}
				<GroupActionBar
					note={data.note}
					color={data.color}
					collapsedByDefault={group?.collapsed_by_default ?? false}
					visible={hover || selected || menuOpen}
					bind:menuOpen
					onAddNote={() => groupEditorContext?.groupEditor.addNote(data.groupId)}
					onRemoveNote={() => groupEditorContext?.groupEditor.removeNote(data.groupId)}
					onUpdateColor={(c) => groupEditorContext?.groupEditor.updateColor(data.groupId, c)}
					onUpdateCollapsedDefault={(v) =>
						groupEditorContext?.groupEditor.updateCollapsedDefault(data.groupId, v)}
					onDeleteGroup={() => groupEditorContext?.groupEditor.deleteGroup(data.groupId)}
				/>
			{/if}
		</div>
	{/snippet}
</NodeWrapper>
