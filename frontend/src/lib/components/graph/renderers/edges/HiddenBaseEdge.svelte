<script lang="ts">
	import { BaseEdge } from '@xyflow/svelte'
	interface Props {
		markerEnd?: string | undefined;
	}

	let { markerEnd = undefined }: Props = $props();
</script>

<BaseEdge path={''} {markerEnd} class="hidden" />
