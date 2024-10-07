<script lang="ts">
	import ScheduleEditor from './ScheduleEditor.svelte'
	import { Button } from './common'
	import { workspaceStore } from '$lib/stores'
	import { ScheduleService, type Schedule } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Calendar } from 'lucide-svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import PrimarySchedule from './PrimarySchedule.svelte'
	import Label from '$lib/components/Label.svelte'
	export let isFlow: boolean
	export let path: string
	export let can_write: boolean

	let scheduleEditor: ScheduleEditor

	let schedule: Schedule | false | undefined = undefined
	export let schedules: Schedule[] | undefined = undefined

	$: path && loadSchedule()
	$: path && loadSchedules()

	export async function loadSchedule() {
		try {
			let exists = await ScheduleService.existsSchedule({
				workspace: $workspaceStore ?? '',
				path
			})
			if (exists) {
				schedule = await ScheduleService.getSchedule({
					workspace: $workspaceStore ?? '',
					path
				})
			} else {
				schedule = false
			}
		} catch (e) {
			console.log('no primary schedule')
		}
	}

	async function loadSchedules() {
		try {
			schedules = (
				await ScheduleService.listSchedules({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).filter((s) => s.path != path)
		} catch (e) {
			console.error('impossible to load schedules')
		}
	}

	async function setScheduleEnabled(path: string, enabled: boolean): Promise<void> {
		try {
			await ScheduleService.setScheduleEnabled({
				path,
				workspace: $workspaceStore!,
				requestBody: { enabled }
			})
			loadSchedule()

			sendUserToast(`Schedule ${enabled ? 'enabled' : 'disabled'}`)
		} catch (err) {
			sendUserToast(`Cannot ` + (enabled ? 'disable' : 'enable') + ` schedule: ${err}`, true)
			loadSchedule()
		}
	}
</script>

<ScheduleEditor
	on:update={() => {
		loadSchedule()
		loadSchedules()
	}}
	bind:this={scheduleEditor}
/>

<div class="flex flex-col gap-4">
	<Button
		on:click={() => scheduleEditor?.openNew(isFlow, path)}
		variant="border"
		color="light"
		size="xs"
		startIcon={{ icon: Calendar }}
	>
		New Schedule
	</Button>

	{#if schedule}
		<PrimarySchedule {schedule} {can_write} {path} {isFlow} {scheduleEditor} {setScheduleEnabled} />
	{:else if schedule == undefined}
		<Skeleton layout={[[6]]} />
	{/if}

	<Label label="Other schedules">
		{#if schedules}
			{#if schedules.length == 0}
				<div class="text-xs text-tertiary"> No other schedules </div>
			{:else}
				<div class="flex flex-col divide-y">
					{#each schedules as schedule (schedule.path)}
						<div class="grid grid-cols-6 text-xs items-center">
							<div class="col-span-3 truncate">{schedule.path}</div>
							<div class="col-span-2 flex flex-row gap-4 flex-nowrap">
								<div>{schedule.schedule}</div>
								<div>{schedule.enabled ? 'on' : 'off'}</div>
							</div>
							<div class="flex justify-end">
								<button
									on:click={() => scheduleEditor?.openEdit(schedule.path, isFlow)}
									class="px-2">Edit</button
								>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{:else}
			<Skeleton layout={[[8]]} />
		{/if}
	</Label>
</div>
