<script lang="ts">
	import Popover from './Popover.svelte'
	import { Badge } from './common'
</script>

<Popover notClickable class="inline-flex items-center">
	{#snippet text()}
		Library script (no exported main function)
	{/snippet}
	<Badge small color="yellow">Library</Badge>
</Popover>
