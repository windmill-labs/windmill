<script lang="ts">
	import { Filter } from 'lucide-svelte'
	import { Button } from '../common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	interface Props {
		filters?: import('svelte').Snippet
	}

	let { filters }: Props = $props()
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	contentClasses="border rounded-lg shadow-lg p-4 flex flex-col w-80 pt-8 bg-surface"
>
	{#snippet trigger()}
		<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: Filter }}>
			Filters
		</Button>
	{/snippet}
	{#snippet content()}
		{@render filters?.()}
	{/snippet}
</Popover>
