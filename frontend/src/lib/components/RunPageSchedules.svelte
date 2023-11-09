<script lang="ts">
	import ScheduleEditor from './ScheduleEditor.svelte'
	import { Badge, Button } from './common'
	import { workspaceStore } from '$lib/stores'
	import { ScheduleService, type Schedule } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Toggle from './Toggle.svelte'
	import { ListOrdered, Calendar, PenBox } from 'lucide-svelte'
	import JobArgs from './JobArgs.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Tooltip from './Tooltip.svelte'

	export let isFlow: boolean
	export let path: string
	export let can_write: boolean

	let scheduleEditor: ScheduleEditor

	let schedule: Schedule | false | undefined = undefined
	let schedules: Schedule[] | undefined = undefined

	$: path && loadSchedule()
	$: path && loadSchedules()

	async function loadSchedule() {
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

<div class="p-2 flex flex-col">
	<Button
		on:click={() => scheduleEditor?.openNew(isFlow, path)}
		variant="border"
		color="light"
		size="xs"
		startIcon={{ icon: Calendar }}
	>
		New Schedule
	</Button>
</div>

{#if schedule}
	<div class="p-2 flex flex-col gap-2">
		<div class="flex flex-row justify-between h-8">
			<div class="flex flex-row gap-2">
				<input
					class="inline-block !w-32"
					type="text"
					id="cron-schedule"
					name="cron-schedule"
					placeholder="*/30 * * * *"
					value={schedule.schedule}
					disabled={true}
				/>
				<Badge color="indigo" small
					>Primary schedule&nbsp;<Tooltip
						>Share the same path as the script or flow it is attached to and its path get renamed
						whenever the source path is renamed</Tooltip
					></Badge
				>
			</div>
			<div class="flex flex-row gap-2">
				<Toggle
					checked={schedule.enabled}
					on:change={(e) => {
						if (can_write) {
							setScheduleEnabled(path, e.detail)
						} else {
							sendUserToast('not enough permission', true)
						}
					}}
					options={{
						right: 'On'
					}}
					size="xs"
				/>
				<Button size="xs" variant="border" color="light" href={`/runs/${path}`}>
					<div class="flex flex-row gap-2">
						<ListOrdered size={14} />
						Runs
					</div>
				</Button>
				<Button
					size="xs"
					color="dark"
					on:click={() => scheduleEditor?.openEdit(path ?? '', isFlow)}
				>
					<PenBox size={14} />
				</Button>
			</div>
		</div>
		{#if Object.keys(schedule?.args ?? {}).length > 0}
			<div class="">
				<JobArgs args={schedule.args ?? {}} />
			</div>
		{:else}
			<div class="text-xs texg-gray-700"> No arguments </div>
		{/if}
	</div>
{:else if schedule == undefined}
	<Skeleton layout={[[6]]} />
{/if}

<h3 class="px-2 pt-4">Other schedules</h3>

{#if schedules}
	{#if schedules.length == 0}
		<div class="text-xs text-secondary px-2"> No other schedules </div>
	{:else}
		<div class="flex flex-col divide-y px-2 pt-2">
			{#each schedules as schedule}
				<div class="grid grid-cols-6 text-2xs items-center py-2"
					><div class="col-span-3 truncate">{schedule.path}</div><div>{schedule.schedule}</div>
					<div>{schedule.enabled ? 'on' : 'off'}</div>
					<button on:click={() => scheduleEditor?.openEdit(path, isFlow)}>Edit</button>
				</div>
			{/each}
		</div>
	{/if}
{:else}
	<Skeleton layout={[[8]]} />
{/if}
