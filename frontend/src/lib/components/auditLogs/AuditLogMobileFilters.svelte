<script lang="ts">
	import { Filter } from 'lucide-svelte'
	import { Button } from '../common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	contentClasses="border rounded-lg shadow-lg p-4 flex flex-col w-80 pt-8 bg-surface"
>
	<svelte:fragment slot="trigger">
		<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: Filter }}>
			Filters
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<slot name="filters" />
	</svelte:fragment>
</Popover>
