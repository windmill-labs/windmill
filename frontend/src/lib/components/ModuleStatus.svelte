<script lang="ts">
	import { FlowStatusModule } from '$lib/gen'
	import { Badge } from './common'
	import { forLater } from '$lib/forLater'
	import { displayDate } from '$lib/utils'
	import { Hourglass } from 'lucide-svelte'

	export let type: FlowStatusModule.type
	export let scheduled_for: Date | undefined
</script>

{#if type == FlowStatusModule.type.WAITING_FOR_EVENTS}
	<span class="italic text-waiting">
		<Hourglass />
		Waiting to be resumed by resume events such as approvals
	</span>
{:else if type == FlowStatusModule.type.WAITING_FOR_PRIOR_STEPS}
	<span class="italic text-tertiary">
		<Hourglass />
		Waiting for prior steps to complete
	</span>
{:else if type == FlowStatusModule.type.WAITING_FOR_EXECUTOR}
	<span class="italic text-tertiary">
		<Hourglass />
		{#if scheduled_for && forLater(scheduled_for.toString())}
			Job is scheduled to be executed at {displayDate(scheduled_for, true)}
		{:else}
			Job is waiting for an executor
		{/if}
	</span>
{:else if type == FlowStatusModule.type.SUCCESS}
	<Badge color="green">Success</Badge>
{:else if type == FlowStatusModule.type.FAILURE}
	<Badge color="red">Failure</Badge>
{/if}

<style>
	.text-waiting {
		color: #c76bf2;
	}
</style>
