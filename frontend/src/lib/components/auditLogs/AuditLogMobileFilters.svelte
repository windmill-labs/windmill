<script lang="ts">
	import { Filter } from 'lucide-svelte'
	import { Popup, Button } from '../common'
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
>
	<svelte:fragment slot="button">
		<Button color="dark" size="xs" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center">
				<Filter size={16} />
				Filters
			</div>
		</Button>
	</svelte:fragment>
	<div class="flex flex-col w-80">
		<slot name="filters" />
	</div>
</Popup>
