<script lang="ts">
	import Popover from './Popover.svelte'
	import { Badge } from './common'

	interface Props {
		has_draft?: boolean
	}

	let { has_draft = false }: Props = $props()
</script>

{#if has_draft}
	<Popover notClickable>
		{#snippet text()}
			Is deployed and has a draft
		{/snippet}
		<Badge small color="indigo">+Draft</Badge>
	</Popover>
{/if}
