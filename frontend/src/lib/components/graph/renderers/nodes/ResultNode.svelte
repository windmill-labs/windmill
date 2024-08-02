<script lang="ts">
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import { Handle, Position, type NodeProps } from '@xyflow/svelte'

	type $$Props = NodeProps

	$$restProps
	let darkMode: boolean = false
</script>

<DarkModeObserver bind:darkMode />

<VirtualItem
	label={'Result'}
	modules={[]}
	index={1}
	selectable
	selected={false}
	insertable={false}
	bgColor={darkMode ? '#2e3440' : '#dfe6ee'}
/>

<Handle type="target" position={Position.Top} />
