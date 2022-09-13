<script lang="ts">
	import CronInput from '$lib/components/CronInput.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'
	import { flowStore } from '../flowStore'
	import type { FlowEditorContext } from '../types'
	const { schedule } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<CronInput bind:schedule={$schedule.cron} />

<SchemaForm schema={$flowStore.schema} bind:args={$schedule.args} />

<Toggle
	bind:checked={$schedule.enabled}
	options={{
		right: 'Schedule enabled'
	}}
/>
