<script lang="ts">
	// UNUSED FOR NOW

	import { Filter } from 'lucide-svelte'
	import { Button } from '../common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	<svelte:fragment slot="trigger">
		<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: Filter }}>
			Filters
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div class="flex flex-col w-80 p-4">
			<slot name="filters" />
		</div>
	</svelte:fragment>
</Popover>
