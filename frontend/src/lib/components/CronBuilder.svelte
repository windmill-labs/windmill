<script lang="ts">
	import Section from './Section.svelte'
	import { Button } from './common'
	import { Clock } from 'lucide-svelte'
	import Popover from './meltComponents/Popover.svelte'
	interface Props {
		children?: import('svelte').Snippet<[any]>
	}

	let { children }: Props = $props()
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }} closeButton>
	{#snippet trigger()}
		<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: Clock }}>
			Use simplified builder
		</Button>
	{/snippet}
	{#snippet content({ close })}
		<Section label="CRON Builder" wrapperClass="p-4">
			<div class="flex flex-col w-72">
				{@render children?.({ close })}
			</div>
		</Section>
	{/snippet}
</Popover>
