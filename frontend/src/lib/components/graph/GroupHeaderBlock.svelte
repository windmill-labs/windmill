<script lang="ts">
	import GroupHeader from './GroupHeader.svelte'
	import GroupNoteArea from './GroupNoteArea.svelte'
	import GroupActionBar from './GroupActionBar.svelte'
	import { getGroupEditorContext } from './groupEditor.svelte'
	import { getGraphContext } from './graphContext'

	interface Props {
		groupId: string
		summary?: string
		note?: string | null
		color?: string
		collapsed: boolean
		autocollapse: boolean
		editMode: boolean
		showNotes: boolean
	}

	let { groupId, summary, note, color, collapsed, autocollapse, editMode, showNotes }: Props =
		$props()

	const groupEditorContext = getGroupEditorContext()
	const graphContext = getGraphContext()
	const moveManager = graphContext?.moveManager

	let moveModuleId = $derived(collapsed ? `collapsed-group:${groupId}` : `group:${groupId}`)

	let hovered = $state(false)
	let menuOpen = $state(false)
	let actionBarHovered = $state(false)

	let visible = $derived(hovered || menuOpen || actionBarHovered)
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="nodrag relative"
	onmouseenter={() => (hovered = true)}
	onmouseleave={() => (hovered = false)}
>
	<GroupHeader
		{summary}
		{color}
		{collapsed}
		{editMode}
		onToggleCollapse={() => graphContext?.groupDisplayState?.toggleRuntimeCollapse(groupId)}
		onSummaryUpdate={(text) => groupEditorContext?.groupEditor.updateSummary(groupId, text)}
	/>
	{#if showNotes && note != null}
		<GroupNoteArea
			note={note ?? ''}
			{color}
			{collapsed}
			{editMode}
			onHeightChange={(h) => graphContext?.groupDisplayState?.setNoteHeight(groupId, h)}
			onNoteUpdate={(text) => groupEditorContext?.groupEditor.updateNote(groupId, text)}
		/>
	{/if}
	{#if editMode}
		<GroupActionBar
			{note}
			{color}
			{autocollapse}
			{visible}
			{menuOpen}
			{moveManager}
			{moveModuleId}
			onMenuOpenChange={(open) => (menuOpen = open)}
			onAddNote={() => groupEditorContext?.groupEditor.addNote(groupId)}
			onRemoveNote={() => groupEditorContext?.groupEditor.removeNote(groupId)}
			onUpdateColor={(c) => groupEditorContext?.groupEditor.updateColor(groupId, c)}
			onUpdateAutocollapse={(v) => groupEditorContext?.groupEditor.updateAutocollapse(groupId, v)}
			onDeleteGroup={() => groupEditorContext?.groupEditor.deleteGroup(groupId)}
		/>
	{/if}
</div>
