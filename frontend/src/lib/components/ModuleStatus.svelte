<script lang="ts">
	import { FlowStatusModule } from '$lib/gen'
	import { faHourglassHalf } from '@fortawesome/free-solid-svg-icons'
	import { Icon } from 'svelte-awesome'
	import { Badge } from './common'

	export let type: FlowStatusModule.type
	export let scheduled_for: string | undefined
</script>

{#if type == FlowStatusModule.type.WAITING_FOR_EVENTS}
	<span class="italic text-waiting">
		<Icon class=" mr-2" data={faHourglassHalf} />
		Waiting to be resumed by resume events such as approvals
	</span>
{:else if type == FlowStatusModule.type.WAITING_FOR_PRIOR_STEPS}
	<span class="italic text-gray-600">
		<Icon data={faHourglassHalf} class="mr-2" />
		Waiting for prior steps to complete
	</span>
{:else if type == FlowStatusModule.type.WAITING_FOR_EXECUTOR}
	<span class="italic text-gray-600">
		<Icon data={faHourglassHalf} class="mr-2" />
		{#if scheduled_for}
			Job has been {scheduled_for} and will be executed after that time
		{:else}
			Job is waiting to be executed
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
