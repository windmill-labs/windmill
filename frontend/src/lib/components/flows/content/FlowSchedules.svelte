<script lang="ts">
	import CronInput from '$lib/components/CronInput.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { emptyString } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'

	const { schedule, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<CronInput edit={true} bind:schedule={$schedule.cron} bind:timezone={$schedule.timezone} />
<SchemaForm schema={$flowStore.schema} bind:args={$schedule.args} />
{#if emptyString($schedule.cron)}
	<p class="text-xs text-gray-600 mt-10">Define a schedule frequency first</p>
{/if}
<Toggle
	disabled={emptyString($schedule.cron)}
	bind:checked={$schedule.enabled}
	options={{
		right: 'Schedule enabled'
	}}
/>
