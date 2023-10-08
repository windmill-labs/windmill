<script lang="ts">
	import Section from './Section.svelte'
	import { Button, Popup } from './common'

	import { Clock } from 'lucide-svelte'
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
	let:close
>
	<svelte:fragment slot="button">
		<Button color="dark" size="xs" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center">
				<Clock size={16} />
				Use simplified builder
			</div>
		</Button>
	</svelte:fragment>
	<Section label="CRON Builder">
		<div class="flex flex-col w-72">
			<slot {close} />
		</div>
	</Section>
</Popup>
