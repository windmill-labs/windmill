<script lang="ts">
	import Popover from './Popover.svelte'
	import { Badge } from './common'
</script>

<Popover notClickable>
	{#snippet text()}
		The script has no main function exported
	{/snippet}
	<Badge small color="yellow" baseClass="border border-indigo-200">No main</Badge>
</Popover>
