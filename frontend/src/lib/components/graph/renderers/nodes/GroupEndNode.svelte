<script lang="ts">
	import { NODE } from '../../util'
	import NodeWrapper from './NodeWrapper.svelte'
</script>

<!-- Invisible layout node for group end boundary. -->
<NodeWrapper>
	<div style="width: {NODE.width}px; height: 1px;"></div>
</NodeWrapper>
