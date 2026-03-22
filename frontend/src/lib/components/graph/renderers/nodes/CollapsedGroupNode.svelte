<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { CollapsedGroupN } from '../../graphBuilder.svelte'
	import GroupModuleIcons from '../../GroupModuleIcons.svelte'
	import GroupHeaderBlock from '../../GroupHeaderBlock.svelte'
	import { NoteColor, NOTE_COLORS } from '../../noteColors'

	interface Props {
		data: CollapsedGroupN['data']
		id: string
	}

	let { data, id }: Props = $props()

	let outlineColorClass = $derived(
		(NOTE_COLORS[(data.color as NoteColor) ?? NoteColor.BLUE] ?? NOTE_COLORS[NoteColor.BLUE])
			.outline
	)

	let bgColorClass = $derived(
		NOTE_COLORS[(data.color as NoteColor) ?? NoteColor.BLUE]?.backgroundLight ??
			NOTE_COLORS[NoteColor.BLUE].backgroundLight
	)
</script>

<NodeWrapper nodeId={id}>
	<div
		class="w-full max-w-full rounded-lg outline outline-1 -outline-offset-1 {outlineColorClass} {bgColorClass}"
		style="width: 275px;"
	>
		<GroupHeaderBlock
			groupId={data.groupId}
			summary={data.summary}
			note={data.note}
			color={data.color}
			collapsed={true}
			collapsedByDefault={data.collapsed_by_default ?? false}
			editMode={data.editMode}
			showNotes={data.showNotes}
		/>
		{#if data.modules && data.modules.length > 0}
			<div class="flex items-center justify-center w-full gap-1.5 px-2 h-[34px]">
				<GroupModuleIcons modules={data.modules} />
			</div>
		{/if}
	</div>
</NodeWrapper>
