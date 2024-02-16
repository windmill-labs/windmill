<script lang="ts">
	import { msToSec } from '$lib/utils'
	import { Badge } from './common'
	import Tooltip from './Tooltip.svelte'
	import { Hourglass } from 'lucide-svelte'

	export let duration_ms: number
	export let flow: boolean
</script>

<Badge large icon={{ icon: Hourglass, position: 'left' }}>
	Ran in {msToSec(duration_ms)}s {#if flow}(sum){/if}
	{#if flow}
		<Tooltip>
			Cumulative time of the execution of each steps. Suspend/sleep/transition times are not
			accounted for, and each step's duration in a parallel branch would be added. Hence the time
			here can differ from the time it took for the flow to have a result from start.
		</Tooltip>
	{/if}
</Badge>
