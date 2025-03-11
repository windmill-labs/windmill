<script lang="ts">
	import type { FlowStatusModule } from '$lib/gen'
	import { Badge } from './common'
	import { forLater } from '$lib/forLater'
	import { displayDate } from '$lib/utils'
	import { Hourglass } from 'lucide-svelte'

	export let type: FlowStatusModule['type']
	export let scheduled_for: Date | undefined
	export let skipped: boolean = false
</script>

{#if type == 'WaitingForEvents'}
	<span class="italic text-violet-700 dark:text-violet-400">
		<Hourglass class="inline" />
		Waiting to be resumed by resume events such as approvals
	</span>
{:else if type == 'WaitingForPriorSteps'}
	<span class="italic text-tertiary">
		<Hourglass class="inline" />
		Waiting for prior steps to complete
	</span>
{:else if type == 'WaitingForExecutor'}
	<span class="italic text-tertiary">
		<Hourglass class="inline" />
		{#if scheduled_for && forLater(scheduled_for.toString())}
			Job is scheduled to be executed at {displayDate(scheduled_for, true)}
		{:else}
			Job is waiting for an executor
		{/if}
	</span>
{:else if skipped}
	<Badge color="blue">Skipped</Badge>
{:else if type == 'Success'}
	<Badge color="green">Success</Badge>
{:else if type == 'Failure'}
	<Badge color="red">Failure</Badge>
{/if}
