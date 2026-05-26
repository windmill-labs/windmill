<script lang="ts">
	import Popover from './Popover.svelte'
	import { Badge } from './common'

	interface Props {
		has_local_changes?: boolean
	}

	let { has_local_changes = false }: Props = $props()
</script>

{#if has_local_changes}
	<Popover notClickable>
		{#snippet text()}
			Has unsaved local changes saved in this browser (not deployed)
		{/snippet}
		<Badge small color="yellow">Local changes</Badge>
	</Popover>
{/if}
