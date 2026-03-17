<script lang="ts">
	import Popover from './Popover.svelte'
	import { Badge } from './common'
</script>

<Popover notClickable>
	{#snippet text()}
		Library script (no exported main function)
	{/snippet}
	<Badge small color="yellow" baseClass="border border-indigo-200">Library</Badge>
</Popover>
