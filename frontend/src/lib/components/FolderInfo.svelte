<script lang="ts">
	import Popover from './Popover.svelte'

	interface Props {
		members: string[]
	}

	let { members }: Props = $props()
</script>

<Popover>
	<div class="inline-flex gap-1 items-center">
		<span class="text-tertiary text-xs">({members.length}) </span>
		<div class="max-w-xs truncate">
			<span class="text-tertiary text-xs">{members?.join(', ')} </span>
		</div>
	</div>
	{#snippet text()}
		<span>{members?.join(', ')} </span>
	{/snippet}
</Popover>
