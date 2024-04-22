<script lang="ts">
	import Popover from './Popover.svelte'
	import { Badge } from './common'
</script>

<Popover notClickable>
	<svelte:fragment slot="text">The script has no main function exported</svelte:fragment>
	<Badge small color="yellow" baseClass="border border-indigo-200">No main</Badge>
</Popover>
