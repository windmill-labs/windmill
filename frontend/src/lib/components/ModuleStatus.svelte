<script lang="ts">
	import type { FlowStatusModule } from '$lib/gen'
	import type { GraphModuleState } from './graph'
	import { Badge } from './common'
	import { forLater } from '$lib/forLater'
	import { displayDate, msToCompactDuration } from '$lib/utils'
	import { Gauge, Hourglass, Moon } from 'lucide-svelte'
	import { useApprovalCount } from './graph/approvalCount.svelte'

	interface Props {
		type: FlowStatusModule['type']
		scheduled_for: Date | undefined
		waiting_reason?: GraphModuleState['waiting_reason']
		concurrency_key?: string
		/** When its concurrency limit first held the step back. */
		held_since?: string
		approval_wait?: GraphModuleState['approval_wait']
		skipped?: boolean
	}

	let {
		type,
		scheduled_for,
		waiting_reason = undefined,
		concurrency_key = undefined,
		held_since = undefined,
		approval_wait = undefined,
		skipped = false
	}: Props = $props()

	const approval = useApprovalCount(() => approval_wait)

	let now = $state(Date.now())
	$effect(() => {
		if (!waiting_reason) return
		const id = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(id)
	})
	let sleepLeft = $derived(Math.max(new Date(scheduled_for ?? now).getTime() - now, 0))
	let heldFor = $derived(held_since ? Math.max(now - new Date(held_since).getTime(), 0) : undefined)
</script>

{#if type == 'WaitingForEvents'}
	<span class="italic text-xs text-primary">
		<Hourglass class="inline text-violet-500" size={14} />
		{approval_wait ? approval.label : 'Waiting to be resumed by resume events such as approvals'}
	</span>
{:else if type == 'WaitingForPriorSteps'}
	<span class="italic text-xs text-primary">
		<Hourglass class="inline" size={14} />
		Waiting for prior steps to complete
	</span>
{:else if type == 'WaitingForExecutor' && waiting_reason === 'sleep'}
	<span class="italic text-xs text-primary">
		<Moon class="inline" size={14} />
		Sleeping until {displayDate(scheduled_for, true)} · {msToCompactDuration(sleepLeft, 0)} left
	</span>
{:else if type == 'WaitingForExecutor' && waiting_reason === 'concurrency'}
	<span class="italic text-xs text-primary">
		<Gauge class="inline text-orange-500" size={14} />
		Held by concurrency limit{heldFor != undefined
			? ` · ${msToCompactDuration(heldFor, 0)}`
			: ''}{concurrency_key ? ` (key ${concurrency_key})` : ''}, next check by {displayDate(
			scheduled_for,
			true
		)}
	</span>
{:else if type == 'WaitingForExecutor'}
	<span class="italic text-xs text-primary">
		<Hourglass class="inline" size={14} />
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
