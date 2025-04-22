<script lang="ts">
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import { Button, Section } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'
	import { ScheduleService, type Schedule } from '$lib/gen'
	import { Trash, Save, Pen, X } from 'lucide-svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import Label from '$lib/components/Label.svelte'
	import { getContext } from 'svelte'
	import type { ScheduleTrigger, TriggerContext } from '$lib/components/triggers'
	import CronInput from '$lib/components/CronInput.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { emptyString, sendUserToast } from '$lib/utils'
	import Toggle from '$lib/components/Toggle.svelte'
	import { loadSchedules, saveSchedule } from '$lib/components/flows/scheduleUtils'
	import { type Writable, writable } from 'svelte/store'
	import Description from '$lib/components/Description.svelte'
	import { createEventDispatcher } from 'svelte'
	import { onMount, tick } from 'svelte'

	export let schema: any
	export let isFlow: boolean
	export let path: string
	export let can_write: boolean
	export let newItem: boolean = false
	export let isNewSchedule: boolean = false
	export let isDeployed: boolean = false

	const { primarySchedule, triggersCount } = getContext<TriggerContext>('TriggerContext')
	const dispatch = createEventDispatcher<{
		update: 'save' | 'delete' | 'update'
	}>()

	let scheduleEditor: ScheduleEditor
	let schedules: Writable<Schedule[] | undefined> = writable(undefined)
	let initialPrimarySchedule: Writable<ScheduleTrigger | false | undefined> = writable(undefined)
	let tmpPrimarySchedule: false | ScheduleTrigger | undefined = undefined // Used to save draft
	let editMode: boolean = false
	let scheduleIsDeployed: Writable<boolean | undefined> = writable(undefined)

	async function updateSchedules(forceRefresh: boolean) {
		const loadPrimarySchedule = true
		await tick()
		loadSchedules(
			forceRefresh,
			path,
			isFlow,
			schedules,
			primarySchedule,
			initialPrimarySchedule,
			$workspaceStore ?? '',
			triggersCount,
			loadPrimarySchedule,
			scheduleIsDeployed
		).then(() => {
			if ($primarySchedule) {
				tmpPrimarySchedule = structuredClone($primarySchedule)
			}
		})
	}

	$: updateSchedules(false) || path

	async function save() {
		if (!tmpPrimarySchedule) return
		$primarySchedule = structuredClone(tmpPrimarySchedule)
		editMode = false
		await saveSchedule(path, newItem, $workspaceStore ?? '', primarySchedule, isFlow)
		updateSchedules(true)
	}

	function saveDraft() {
		if (!tmpPrimarySchedule) return
		const firstSave = !$primarySchedule
		$primarySchedule = structuredClone(tmpPrimarySchedule)
		if (firstSave) {
			$triggersCount = {
				...($triggersCount ?? {}),
				schedule_count: ($triggersCount?.schedule_count ?? 0) + 1,
				primary_schedule: { schedule: tmpPrimarySchedule?.cron }
			}
			dispatch('update', 'save')
		} else {
			dispatch('update', 'update')
		}
		editMode = false
	}

	function deleteDraft() {
		dispatch('update', 'delete')
	}

	function deletePendingPrimarySchedule() {
		$primarySchedule = false
		$triggersCount = {
			...($triggersCount ?? {}),
			schedule_count: ($triggersCount?.schedule_count ?? 1) - 1,
			primary_schedule: undefined
		}
		dispatch('update', 'delete')
	}

	export function setNewSchedule() {
		tmpPrimarySchedule = {
			summary: '',
			args: {},
			cron: '0 0 */1 * * *',
			timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
			enabled: true
		}
		editMode = true
	}

	onMount(() => {
		if (isNewSchedule) {
			setNewSchedule()
		}
	})
</script>

<Section label="Primary schedule" class="flex flex-col gap-4 w-full">
	<svelte:fragment slot="header">
		{#if !$scheduleIsDeployed && !isDeployed && $primarySchedule}
			<span
				class="ml-1 bg-blue-50 dark:bg-blue-900/40 px-2 py-1 rounded text-xs font-normal text-blue-700 dark:text-blue-100"
			>
				Will be deployed automatically with the flow
			</span>
		{/if}
	</svelte:fragment>
	<svelte:fragment slot="action">
		<div class="flex flex-row gap-2 items-center">
			{#if can_write && !$scheduleIsDeployed}
				<Button
					on:click={() => {
						if (isNewSchedule) {
							deleteDraft()
						} else {
							deletePendingPrimarySchedule()
						}
					}}
					btnClasses="hover:bg-red-500 hover:text-white"
					color="light"
					size="xs"
					startIcon={{ icon: Trash }}
				/>
			{/if}
			{#if $primarySchedule && !newItem}
				<Toggle
					disabled={emptyString($primarySchedule.cron) || !can_write}
					bind:checked={$primarySchedule.enabled}
					options={{
						right: 'Enabled'
					}}
					size="xs"
					on:change={async (e) => {
						if ($scheduleIsDeployed) {
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
			{#if !editMode && can_write}
				<Button
					size="xs"
					color="light"
					startIcon={{ icon: Pen }}
					on:click={() => {
						tmpPrimarySchedule = structuredClone($primarySchedule)
						editMode = true
					}}>Edit</Button
				>
			{:else if editMode && !$primarySchedule && !isDeployed}
				<Button size="xs" startIcon={{ icon: Save }} on:click={saveDraft}>Deploy with flow</Button>
			{:else if editMode}
				<Button
					size="xs"
					startIcon={{ icon: Save }}
					on:click={() => {
						if (isDeployed) {
							save()
						} else {
							saveDraft()
						}
					}}
					disabled={JSON.stringify({ ...$primarySchedule, enabled: true }) ==
						JSON.stringify({ ...tmpPrimarySchedule, enabled: true })}
				>
					Save
				</Button>
			{/if}
			{#if editMode && $primarySchedule}
				<Button
					size="xs"
					color="light"
					on:click={() => {
						tmpPrimarySchedule = structuredClone($primarySchedule)
						editMode = false
					}}
					startIcon={{ icon: X }}
				>
					Cancel
				</Button>
			{/if}
		</div>
	</svelte:fragment>

	<Description link="https://www.windmill.dev/docs/core_concepts/scheduling">
		Run scripts and flows automatically on a recurring basis using cron expressions. The primary
		schedule shares the path of the script or flow.
	</Description>

	<ScheduleEditor
		on:update={() => {
			updateSchedules(true)
		}}
		bind:this={scheduleEditor}
	/>

	{#if $primarySchedule == undefined}
		<Skeleton layout={[[12]]} />
	{:else if tmpPrimarySchedule}
		<div class="w-full flex flex-col mb-4">
			<!-- svelte-ignore a11y-autofocus -->
			<div class="mt-5">
				<Label label="Summary" class="font-semibold" primary>
					<input
						autofocus
						type="text"
						placeholder="Short summary to be displayed when listed"
						class="text-sm w-full"
						bind:value={tmpPrimarySchedule.summary}
						disabled={!editMode}
					/>
				</Label>
			</div>
		</div>
		<CronInput
			bind:schedule={tmpPrimarySchedule.cron}
			bind:timezone={tmpPrimarySchedule.timezone}
			disabled={!editMode}
		/>
		<SchemaForm
			onlyMaskPassword
			{schema}
			bind:args={tmpPrimarySchedule.args}
			disabled={!editMode}
		/>
		{#if emptyString(tmpPrimarySchedule.cron) && editMode}
			<p class="text-xs text-tertiary mt-10">Define a schedule frequency first</p>
		{/if}

		{#if $scheduleIsDeployed}
			<div class="flex">
				<Button
					size="sm"
					color="light"
					variant="border"
					on:click={() => scheduleEditor?.openEdit(path, isFlow)}>Advanced</Button
				>
			</div>
		{/if}
	{/if}
</Section>
