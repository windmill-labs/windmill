<script lang="ts">
	import { NODE } from '../../util'
	import NodeWrapper from './NodeWrapper.svelte'

	interface Props {
		id: string
	}

	let { id }: Props = $props()
</script>

<!-- Invisible layout node for group end boundary. -->
<NodeWrapper nodeId={id}>
	<div style="width: {NODE.width}px; height: 1px;"></div>
</NodeWrapper>
