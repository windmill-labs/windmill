<script lang="ts">
	import ScheduleEditor from './ScheduleEditor.svelte'
	import { Button } from './common'
	import { workspaceStore } from '$lib/stores'
	import { ScheduleService, type Schedule } from '$lib/gen'
	import { Calendar, Trash, Save } from 'lucide-svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Label from '$lib/components/Label.svelte'
	import { getContext } from 'svelte'
	import type { TriggerContext } from './triggers'
	import CronInput from './CronInput.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import { emptyString } from '$lib/utils'
	import Toggle from './Toggle.svelte'

	export let schema: any
	export let isFlow: boolean
	export let path: string
	export let can_write: boolean
	export let newItem: boolean = false

	const { primarySchedule } = getContext<TriggerContext>('TriggerContext')

	let scheduleEditor: ScheduleEditor

	$: path && loadSchedules()

	let schedules: Schedule[] | undefined = undefined
	async function loadSchedules() {
		try {
			const allSchedules = await ScheduleService.listSchedules({
				workspace: $workspaceStore ?? '',
				path: path,
				isFlow: true
			})
			const primary = allSchedules.find((s) => s.path == path)
			if (primary) {
				$primarySchedule = {
					summary: primary.summary,
					args: primary.args ?? {},
					cron: primary.schedule,
					timezone: primary.timezone,
					enabled: primary.enabled
				}
			} else if (!$primarySchedule) {
				$primarySchedule = false
			}
			schedules = allSchedules.filter((s) => s.path != path)
		} catch (e) {
			console.error('impossible to load schedules')
		}
	}
</script>

<ScheduleEditor
	on:update={() => {
		loadSchedules()
	}}
	bind:this={scheduleEditor}
/>

<div class="flex flex-col gap-4 w-full">
	{#if $primarySchedule}
		<div class="w-full flex flex-col mb-4">
			<div class="w-full flex-row-reverse justify-between flex mb-2">
				<Button
					on:click={() => {
						$primarySchedule = false
					}}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: Trash }}
				/>
				{#if !newItem}
					<Button color="dark" size="sm" startIcon={{ icon: Save }}>Save changes</Button>
				{/if}
			</div>
			<!-- svelte-ignore a11y-autofocus -->
			<Label label="Summary">
				<input
					autofocus
					type="text"
					placeholder="Short summary to be displayed when listed"
					class="text-sm w-full"
					bind:value={$primarySchedule.summary}
				/>
			</Label>
		</div>
		<CronInput bind:schedule={$primarySchedule.cron} bind:timezone={$primarySchedule.timezone} />
		<SchemaForm {schema} bind:args={$primarySchedule.args} />
		{#if emptyString($primarySchedule.cron)}
			<p class="text-xs text-tertiary mt-10">Define a schedule frequency first</p>
		{/if}
		<Toggle
			disabled={emptyString($primarySchedule.cron)}
			bind:checked={$primarySchedule.enabled}
			options={{
				right: 'Schedule enabled'
			}}
		/>
	{:else}
		<Button
			btnClasses="mt-2"
			on:click={() => {
				$primarySchedule = {
					summary: '',
					args: {},
					cron: '0 0 0 /1 * * *',
					timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
					enabled: true
				}
			}}
			variant="border"
			color="light"
			size="md"
			startIcon={{ icon: Calendar }}
		>
			Set Primary Schedule
		</Button>

		<CronInput schedule={''} disabled timezone={Intl.DateTimeFormat().resolvedOptions().timeZone} />

		<SchemaForm disabled {schema} />
	{/if}

	{#if !newItem}
		<div class="mt-10" />
		{#if $primarySchedule}
			<Button
				on:click={() => scheduleEditor?.openNew(isFlow, path)}
				variant="border"
				color="light"
				size="xs"
				startIcon={{ icon: Calendar }}
			>
				New Schedule
			</Button>
		{/if}

		<Label label="Other schedules">
			{#if schedules}
				{#if schedules?.length == 0 || $schedules == undefined}
					<div class="text-xs text-tertiary"> No other schedules </div>
				{:else}
					<div class="flex flex-col divide-y">
						{#each schedules as schedule (schedule.path)}
							<div class="grid grid-cols-6 text-xs items-center py-2">
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
	{/if}
</div>
