<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, Position, type NodeProps } from '@xyflow/svelte'
	import NodeWrapper from './NodeWrapper.svelte'
</script>

<NodeWrapper let:darkMode enableSourceHandle={false}>
	<VirtualItem
		label={'Result'}
		modules={[]}
		index={1}
		selectable
		selected={false}
		insertable={false}
		bgColor={darkMode ? '#2e3440' : '#dfe6ee'}
	/>
</NodeWrapper>
