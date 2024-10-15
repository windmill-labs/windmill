<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import TriggersWrapper from '../triggers/TriggersWrapper.svelte'
	import { type GraphEventHandlers } from '../../graphBuilder'

	export let data: {
		path: string
		isEditor: boolean
		newFlow: boolean
		extra_perms: Record<string, any>
		eventHandlers: GraphEventHandlers
	}
</script>

<NodeWrapper wrapperClass="shadow-none">
	<TriggersWrapper
		path={data.path}
		on:select={() => {
			data?.eventHandlers?.select('triggers')
		}}
		isFlow={true}
		newItem={data.newFlow}
	/>
</NodeWrapper>
