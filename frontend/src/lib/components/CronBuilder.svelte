<script lang="ts">
	import Section from './Section.svelte'
	import { Button } from './common'
	import { Clock } from 'lucide-svelte'
	import Popover from './meltComponents/Popover.svelte'
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }} closeButton>
	<svelte:fragment slot="trigger">
		<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: Clock }}>
			Use simplified builder
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="content" let:close>
		<Section label="CRON Builder" wrapperClass="p-4">
			<div class="flex flex-col w-72">
				<slot {close} />
			</div>
		</Section>
	</svelte:fragment>
</Popover>
