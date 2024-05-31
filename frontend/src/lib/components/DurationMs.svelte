<script lang="ts">
	import { msToSec } from '$lib/utils'
	import { Badge } from './common'
	import { Hourglass } from 'lucide-svelte'
	import WaitTimeWarning from './common/waitTimeWarning/WaitTimeWarning.svelte'

	export let duration_ms: number
	export let waiting_time_ms: number | undefined = undefined
</script>

<div>
	<Badge large icon={{ icon: Hourglass, position: 'left' }}>
		Ran in {msToSec(duration_ms)}s
	</Badge>
	{#if waiting_time_ms}
		<WaitTimeWarning {waiting_time_ms} variant="badge" />
	{/if}
</div>
