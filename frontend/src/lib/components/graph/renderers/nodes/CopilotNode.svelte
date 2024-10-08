<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import FlowCopilotButton from '$lib/components/flows/map/FlowCopilotButton.svelte'
	import { NODE } from '../../util'
</script>

<NodeWrapper wrapperClass="shadow-none">
	<div style={`width: ${NODE.width}px;`}>
		<FlowCopilotButton />
	</div>
</NodeWrapper>
