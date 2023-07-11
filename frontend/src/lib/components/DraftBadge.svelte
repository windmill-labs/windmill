<script lang="ts">
	import Popover from './Popover.svelte'
	import { Badge } from './common'

	export let has_draft: boolean = false
	export let draft_only: boolean = false
</script>

{#if has_draft}
	{#if draft_only}
		<Popover notClickable>
			<svelte:fragment slot="text">Never deployed and is only a draft</svelte:fragment>
			<Badge small color="indigo" baseClass="border border-indigo-200">Draft only</Badge>
		</Popover>
	{:else}
		<Popover notClickable>
			<svelte:fragment slot="text">Is deployed and has a draft</svelte:fragment>
			<Badge small color="indigo" baseClass="border border-indigo-200">+Draft</Badge>
		</Popover>
	{/if}
{/if}
