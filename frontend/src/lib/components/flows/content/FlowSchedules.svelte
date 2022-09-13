<script lang="ts">
	import CronInput from '$lib/components/CronInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	const { schedule } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<CronInput bind:schedule={$schedule.cron} />

<Toggle
	bind:checked={$schedule.enabled}
	options={{
		right: 'Schedule enabled'
	}}
/>
