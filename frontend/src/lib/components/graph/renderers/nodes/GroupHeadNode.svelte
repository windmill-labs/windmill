<script lang="ts">
	import { NODE } from '../../util'
	import NodeWrapper from './NodeWrapper.svelte'
	import GroupHeaderBlock from '../../GroupHeaderBlock.svelte'
	import type { GroupHeadN } from '../../graphBuilder.svelte'

	interface Props {
		data: GroupHeadN['data']
		id: string
	}

	let { data, id }: Props = $props()
</script>

<NodeWrapper enableSourceHandle enableTargetHandle nodeId={id}>
	<div style="width: {NODE.width}px;">
		<GroupHeaderBlock
			groupId={data.groupId}
			summary={data.summary}
			note={data.note}
			color={data.color}
			collapsed={false}
			autocollapse={data.autocollapse ?? false}
			editMode={data.editMode}
			showNotes={data.showNotes}
		/>
	</div>
</NodeWrapper>
