<script lang="ts">
	import type { FlowStatusModule } from '$lib/gen'
	import type { GraphModuleState } from './graph'
	import { Badge } from './common'
	import { forLater } from '$lib/forLater'
	import { displayDate } from '$lib/utils'
	import { Hourglass, Lock, Timer } from 'lucide-svelte'

	interface Props {
		type: FlowStatusModule['type']
		scheduled_for: Date | undefined
		waiting_reason?: GraphModuleState['waiting_reason']
		concurrency_key?: string
		skipped?: boolean
	}

	let {
		type,
		scheduled_for,
		waiting_reason = undefined,
		concurrency_key = undefined,
		skipped = false
	}: Props = $props()
</script>

{#if type == 'WaitingForEvents'}
	<span class="italic text-xs text-violet-700 dark:text-violet-400">
		<Hourglass class="inline" size={14} />
		Waiting to be resumed by resume events such as approvals
	</span>
{:else if type == 'WaitingForPriorSteps'}
	<span class="italic text-xs text-primary">
		<Hourglass class="inline" size={14} />
		Waiting for prior steps to complete
	</span>
{:else if type == 'WaitingForExecutor'}
	<span class="italic text-xs text-primary">
		{#if waiting_reason === 'sleep'}
			<Timer class="inline" size={14} />
			Sleeping after the previous step until {displayDate(scheduled_for, true)}
		{:else if waiting_reason === 'concurrency'}
			<Lock class="inline" size={14} />
			Held back by its concurrency limit{concurrency_key ? ` (key ${concurrency_key})` : ''}, next
			check by {displayDate(scheduled_for, true)}
		{:else if scheduled_for && forLater(scheduled_for.toString())}
			<Hourglass class="inline" size={14} />
			Job is scheduled to be executed at {displayDate(scheduled_for, true)}
		{:else}
			<Hourglass class="inline" size={14} />
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
