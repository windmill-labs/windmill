<script lang="ts">
	import Popover from './Popover.svelte'
	import { Badge } from './common'

	interface Props {
		has_draft?: boolean
		draft_only?: boolean
	}

	let { has_draft = false, draft_only = false }: Props = $props()
</script>

{#if has_draft}
	{#if draft_only}
		<Popover notClickable>
			{#snippet text()}
				Never deployed and is only a draft
			{/snippet}
			<Badge small color="indigo" baseClass="border border-indigo-200">Draft only</Badge>
		</Popover>
	{:else}
		<Popover notClickable>
			{#snippet text()}
				Is deployed and has a draft
			{/snippet}
			<Badge small color="indigo" baseClass="border border-indigo-200">+Draft</Badge>
		</Popover>
	{/if}
{/if}
