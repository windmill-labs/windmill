<script lang="ts">
	// Renders a small status pill on home-page rows when the authed user
	// has a per-user draft on the entity. `is_draft` is the per-user
	// signal that replaced main's workspace-wide `has_draft` (the field
	// rename mirrors the get-by-path overlay's `is_draft` flag).
	//
	//   draft_only=true  → "Draft only"  (entity has never been deployed)
	//   draft_only=false → "+Draft"      (deployed and user has a draft on top)
	//
	// Nothing renders when `is_draft` is false.
	import Popover from './Popover.svelte'
	import { Badge } from './common'

	interface Props {
		is_draft?: boolean
		draft_only?: boolean
	}

	let { is_draft = false, draft_only = false }: Props = $props()
</script>

{#if is_draft}
	{#if draft_only}
		<Popover notClickable>
			{#snippet text()}
				Never deployed and is only a draft
			{/snippet}
			<Badge small color="indigo">Draft only</Badge>
		</Popover>
	{:else}
		<Popover notClickable>
			{#snippet text()}
				Is deployed and has a draft
			{/snippet}
			<Badge small color="indigo">+Draft</Badge>
		</Popover>
	{/if}
{/if}
