<script lang="ts">
	// UNUSED FOR NOW

	import { Filter } from 'lucide-svelte'
	import { Button } from '../common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	interface Props {
		filters?: import('svelte').Snippet;
	}

	let { filters }: Props = $props();
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	{#snippet trigger()}
	
			<Button variant="accent" size="xs" nonCaptureEvent={true} startIcon={{ icon: Filter }}>
				Filters
			</Button>
		
	{/snippet}
	{#snippet content()}
	
			<div class="flex flex-col w-80 p-4">
				{@render filters?.()}
			</div>
		
	{/snippet}
</Popover>
