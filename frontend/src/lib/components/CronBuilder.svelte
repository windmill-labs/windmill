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
		<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: Clock }}>
			Use simplified builder
		</Button>
	</svelte:fragment>
	<Section label="CRON Builder">
		<div class="flex flex-col w-72">
			<slot {close} />
		</div>
	</Section>
</Popup>
