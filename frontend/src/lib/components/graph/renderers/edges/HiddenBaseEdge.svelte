<script lang="ts">
	import { BaseEdge } from '@xyflow/svelte'
	export let markerEnd: string | undefined = undefined
</script>

<BaseEdge path={''} {markerEnd} class="hidden" />
