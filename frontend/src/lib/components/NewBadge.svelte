<script lang="ts">
	import Popover from './Popover.svelte'
	import { Badge } from './common'

	interface Props {
		is_new?: boolean
	}

	let { is_new = false }: Props = $props()
</script>

{#if is_new}
	<Popover notClickable>
		{#snippet text()}
			Exists only in this browser and has never been saved to the workspace
		{/snippet}
		<Badge small color="blue">New</Badge>
	</Popover>
{/if}
