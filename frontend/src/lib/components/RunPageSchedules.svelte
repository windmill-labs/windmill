<script lang="ts">
	import ScheduleEditor from './ScheduleEditor.svelte'
	import { Button } from './common'
	import { workspaceStore } from '$lib/stores'
	import { ScheduleService, type Schedule } from '$lib/gen'
	import { Calendar, Trash, Save } from 'lucide-svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Label from '$lib/components/Label.svelte'
	import { getContext } from 'svelte'
	import type { ScheduleTrigger, TriggerContext } from './triggers'
	import CronInput from './CronInput.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import { emptyString, sendUserToast } from '$lib/utils'
	import Toggle from './Toggle.svelte'

	export let schema: any
	export let isFlow: boolean
	export let path: string
	export let can_write: boolean
	export let newItem: boolean = false

	const { primarySchedule, triggersCount } = getContext<TriggerContext>('TriggerContext')

	let scheduleEditor: ScheduleEditor

	$: loadSchedules(false) || path

	let schedules: Schedule[] | undefined = undefined
	let initialPrimarySchedule: ScheduleTrigger | false | undefined = undefined
	async function loadSchedules(forceRefresh: boolean) {
		if (!path || path == '') {
			schedules = []
			if ($primarySchedule == undefined) {
				$primarySchedule = false
			}
			initialPrimarySchedule = structuredClone($primarySchedule)
			return
		}
		try {
			const allSchedules = await ScheduleService.listSchedules({
				workspace: $workspaceStore ?? '',
				path: path,
				isFlow
			})
			const primary = allSchedules.find((s) => s.path == path)
			let remotePrimarySchedule = primary
				? {
						summary: primary.summary,
						args: primary.args ?? {},
						cron: primary.schedule,
						timezone: primary.timezone,
						enabled: primary.enabled
				  }
				: false
			if ($primarySchedule == undefined || forceRefresh) {
				$primarySchedule = remotePrimarySchedule
			}
			initialPrimarySchedule = structuredClone(remotePrimarySchedule)

			$triggersCount = {
				...($triggersCount ?? {}),
				schedule_count: allSchedules.length,
				primary_schedule: $primarySchedule ? { schedule: $primarySchedule.cron } : undefined
			}
			schedules = allSchedules.filter((s) => s.path != path)
		} catch (e) {
			console.error('impossible to load schedules', e)
		}
	}

	async function save() {
		const scheduleExists =
			path != '' &&
			!newItem &&
			(await ScheduleService.existsSchedule({
				workspace: $workspaceStore!,
				path
			}))
		if (scheduleExists) {
			console.log('primary schedule exists')
			if ($primarySchedule) {
				await ScheduleService.updateSchedule({
					workspace: $workspaceStore!,
					path,
					requestBody: {
						summary: $primarySchedule.summary,
						args: $primarySchedule.args,
						schedule: $primarySchedule.cron,
						timezone: $primarySchedule.timezone
					}
				})
				sendUserToast(`Primary schedule updated`)
			} else {
				await ScheduleService.deleteSchedule({ workspace: $workspaceStore!, path })
				sendUserToast(`Primary schedule deleted`)
			}
		} else {
			if ($primarySchedule) {
				await ScheduleService.createSchedule({
					workspace: $workspaceStore!,
					requestBody: {
						path,
						script_path: path,
						is_flow: isFlow,
						summary: $primarySchedule.summary,
						args: $primarySchedule.args,
						schedule: $primarySchedule.cron,
						timezone: $primarySchedule.timezone,
						enabled: $primarySchedule.enabled
					}
				})
				sendUserToast(`Primary schedule created`)
			}
		}
		loadSchedules(true)
	}
</script>

<ScheduleEditor
	on:update={() => {
		loadSchedules(true)
	}}
	bind:this={scheduleEditor}
/>

<div class="flex flex-col gap-4 w-full">
	{#if $primarySchedule == undefined}
		<Skeleton layout={[[12]]} />
	{:else if $primarySchedule}
		<div class="w-full flex flex-col mb-4">
			{#if can_write}
				<div class="w-full flex-row-reverse flex mb-2">
					<div class="flex flex-row gap-4">
						<Button
							on:click={() => {
								$primarySchedule = false
								$triggersCount = {
									...($triggersCount ?? {}),
									schedule_count: ($triggersCount?.schedule_count ?? 1) - 1,
									primary_schedule: undefined
								}
							}}
							variant="border"
							color="light"
							size="xs"
							startIcon={{ icon: Trash }}
						/>
						{#if initialPrimarySchedule && !newItem}
							<Toggle
								disabled={emptyString($primarySchedule.cron)}
								bind:checked={$primarySchedule.enabled}
								options={{
									right: 'Enabled'
								}}
								on:change={async (e) => {
									if (!newItem) {
										await ScheduleService.setScheduleEnabled({
											path: path,
											workspace: $workspaceStore ?? '',
											requestBody: { enabled: e.detail }
										})

										sendUserToast(`${e.detail ? 'enabled' : 'disabled'} schedule ${path}`)
									}
								}}
							/>
						{/if}

						{#if !newItem}
							<Button
								on:click={save}
								color="dark"
								size="sm"
								startIcon={{ icon: Save }}
								disabled={JSON.stringify({ ...$primarySchedule, enabled: true }) ==
									JSON.stringify({ ...initialPrimarySchedule, enabled: true })}
								>Apply changes now</Button
							>
						{:else}
							<div class="text-sm text-secondary mt-1 text-center"
								>Deployed automatically with {isFlow ? 'flow' : 'script'}</div
							>
						{/if}
					</div>
				</div>
			{/if}

			<!-- svelte-ignore a11y-autofocus -->
			<div class="mt-5">
				<Label label="Summary" class="font-semibold" primary>
					<input
						autofocus
						type="text"
						placeholder="Short summary to be displayed when listed"
						class="text-sm w-full"
						bind:value={$primarySchedule.summary}
					/>
				</Label>
			</div>
		</div>
		<CronInput bind:schedule={$primarySchedule.cron} bind:timezone={$primarySchedule.timezone} />
		<SchemaForm {schema} bind:args={$primarySchedule.args} />
		{#if emptyString($primarySchedule.cron)}
			<p class="text-xs text-tertiary mt-10">Define a schedule frequency first</p>
		{/if}

		{#if initialPrimarySchedule != false}
			<div class="flex">
				<Button size="xs" color="light" on:click={() => scheduleEditor?.openEdit(path, isFlow)}
					>Advanced</Button
				>
			</div>
		{/if}
	{:else}
		<div class="flex flex-row gap-4 mt-2">
			<Button
				on:click={() => {
					$primarySchedule = {
						summary: '',
						args: {},
						cron: '0 0 */1 * * *',
						timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
						enabled: true
					}
					$triggersCount = {
						...($triggersCount ?? {}),
						schedule_count: ($triggersCount?.schedule_count ?? 0) + 1,
						primary_schedule: { schedule: $primarySchedule.cron }
					}
				}}
				variant="contained"
				color="dark"
				size="md"
				startIcon={{ icon: Calendar }}
			>
				Set Primary Schedule
			</Button>
			{#if initialPrimarySchedule != undefined && initialPrimarySchedule != false && !newItem}
				<Button on:click={save} color="dark" size="md" startIcon={{ icon: Save }}>
					Apply changes now
				</Button>
			{:else}
				<div class="text-sm text-center text-secondary mt-2"
					>Deployed automatically with {isFlow ? 'flow' : 'script'}</div
				>
			{/if}
		</div>

		<Label label="Summary" class="font-semibold" primary>
			<input
				type="text"
				disabled
				placeholder="Short summary to be displayed when listed"
				class="text-sm w-full"
			/>
		</Label>
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
				{#if schedules?.length == 0 || schedules == undefined}
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
