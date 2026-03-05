<script lang="ts">
	import { msToReadableTime } from '$lib/utils'
	import { Badge } from './common'
	import { Hourglass } from 'lucide-svelte'
	import WaitTimeWarning from './common/waitTimeWarning/WaitTimeWarning.svelte'

	interface Props {
		duration_ms: number;
		self_wait_time_ms?: number | undefined;
		aggregate_wait_time_ms?: number | undefined;
	}

	let { duration_ms, self_wait_time_ms = undefined, aggregate_wait_time_ms = undefined }: Props = $props();
</script>

<div>
	<Badge large icon={{ icon: Hourglass, position: 'left' }}>
		Ran in {msToReadableTime(duration_ms)}
		{#if self_wait_time_ms || aggregate_wait_time_ms}
			<WaitTimeWarning {self_wait_time_ms} {aggregate_wait_time_ms} variant="alert" />
		{/if}
	</Badge>
</div>
