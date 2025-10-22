<script lang="ts">
	import { Button } from './common'
	import { Clock } from 'lucide-svelte'
	import Popover from './meltComponents/Popover.svelte'
	interface Props {
		children?: import('svelte').Snippet<[any]>
	}

	let { children }: Props = $props()
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	closeButton
	contentClasses="p-4"
>
	{#snippet trigger()}
		<Button variant="accent" size="xs" nonCaptureEvent={true} startIcon={{ icon: Clock }}>
			Cron builder
		</Button>
	{/snippet}
	{#snippet content({ close })}
		<div class="flex flex-col w-72">
			{@render children?.({ close })}
		</div>
	{/snippet}
</Popover>
