<script lang="ts">
	import { Calendar, Mail, Webhook } from 'lucide-svelte'
	import TriggerButton from './TriggerButton.svelte'
	import { NODE } from '../../util'
	import { ScheduleService, type Schedule } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	export let path: string

	let schedules: Schedule[] = []

	async function loadSchedules() {
		if (!path) return

		try {
			schedules = await ScheduleService.listSchedules({
				workspace: $workspaceStore ?? '',
				path,
				isFlow: true
			})
			console.log(schedules)
		} catch (e) {
			console.error('impossible to load schedules')
		}
	}

	loadSchedules()
</script>

<div style={`width: ${NODE.width}px;`}>
	<div class="flex flex-row gap-4 border p-1 rounded-md border-surface-selected mx-auto w-min">
		<TriggerButton
			on:click={() => {
				alert('webhook')
			}}
		>
			<Webhook size={12} />
		</TriggerButton>

		<TriggerButton>
			<Mail size={12} />
		</TriggerButton>
		{#each schedules as schedule}
			<TriggerButton
				on:click={() => {
					console.log(schedule, 'calendar')
				}}
			>
				<Calendar size={12} />
			</TriggerButton>
		{/each}
	</div>
</div>
