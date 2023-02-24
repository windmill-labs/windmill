<script context="module">
	export function load() {
		return {
			stuff: { title: 'New Schedule' }
		}
	}
</script>

<script lang="ts">
	import { sendUserToast, formatCron, canWrite, emptyString } from '$lib/utils'
	import { ScriptService, Script, ScheduleService, type Flow, FlowService } from '$lib/gen'
	import Toggle from '$lib/components/Toggle.svelte'

	import Path from '$lib/components/Path.svelte'
	import { Alert, Badge, Button } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import CronInput, { OFFSET } from '$lib/components/CronInput.svelte'
	import { faSave } from '@fortawesome/free-solid-svg-icons'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { createEventDispatcher } from 'svelte'

	let initialPath = ''
	let edit = true
	let schedule: string = '0 0 12 * *'

	let itemKind: 'flow' | 'script' = 'script'

	let script_path = ''
	let initialScriptPath = ''

	export function openEdit(ePath: string, isFlow: boolean) {
		is_flow = isFlow
		initialPath = ePath
		itemKind = is_flow ? 'flow' : 'script'
		path = ePath
		edit = true
		drawer?.openDrawer()
	}

	export function openNew(is_flow: boolean, initial_script_path?: string) {
		edit = false
		itemKind = is_flow ? 'flow' : 'script'
		initialScriptPath = initial_script_path ?? ''
		path = initialScriptPath
		initialPath = initialScriptPath
		script_path = initialScriptPath
		drawer?.openDrawer()
	}

	$: is_flow = itemKind == 'flow'

	let runnable: Script | Flow | undefined
	let args: Record<string, any> = {}

	let isValid = true

	let path: string = ''
	let enabled: boolean = false
	let pathError = ''

	let validCRON = true
	$: allowSchedule = isValid && validCRON && script_path != ''

	$: script_path != '' && loadScript(script_path)

	const dispatch = createEventDispatcher()

	async function loadScript(p: string | undefined): Promise<void> {
		if (p) {
			if (is_flow) {
				runnable = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path: p })
			} else {
				runnable = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: p })
			}
		} else {
			runnable = undefined
		}
	}

	let can_write = true
	async function loadSchedule(): Promise<void> {
		try {
			const s = await ScheduleService.getSchedule({
				workspace: $workspaceStore!,
				path: initialPath
			})
			enabled = s.enabled
			schedule = s.schedule
			script_path = s.script_path ?? ''
			is_flow = s.is_flow
			args = s.args ?? {}
			can_write = canWrite(s.path, s.extra_perms, $userStore)
		} catch (err) {
			sendUserToast(`Could not load schedule: ${err}`, true)
		}
	}

	async function scheduleScript(): Promise<void> {
		if (edit) {
			await ScheduleService.updateSchedule({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					schedule: formatCron(schedule),
					args
				}
			})
			sendUserToast(`Schedule ${path} updated`)
		} else {
			await ScheduleService.createSchedule({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					schedule: formatCron(schedule),
					offset: OFFSET,
					script_path,
					is_flow,
					args,
					enabled: true
				}
			})
			sendUserToast(`Schedule ${path} created`)
		}
		dispatch('update')
		drawer.closeDrawer()
	}

	$: {
		if ($workspaceStore) {
			if (edit && path != '') {
				loadSchedule()
			}
		}
	}

	let drawer: Drawer
</script>

<Drawer size="900px" bind:this={drawer}>
	<DrawerContent
		title={edit ? `Edit ${initialPath}` : 'New schedule'}
		on:close={drawer.closeDrawer}
	>
		<svelte:fragment slot="actions">
			{#if edit}
				<div class="mr-8">
					<Toggle
						disabled={!can_write}
						checked={enabled}
						options={{ right: 'enable', left: 'disable' }}
						on:change={async (e) => {
							await ScheduleService.setScheduleEnabled({
								path: initialPath,
								workspace: $workspaceStore ?? '',
								requestBody: { enabled: e.detail }
							})
							sendUserToast(`${e.detail ? 'enabled' : 'disabled'} schedule ${initialPath}`)
						}}
					/>
				</div>
			{/if}
			<Button
				startIcon={{ icon: faSave }}
				disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
				on:click={scheduleScript}
			>
				{edit ? 'Save' : 'Schedule'}
			</Button>
		</svelte:fragment>
		<div>
			{#if !edit}
				<span class="font-semibold text-gray-700">Path</span>
				<Path
					checkInitialPathExistence
					bind:error={pathError}
					bind:path
					{initialPath}
					namePlaceholder="schedule"
					kind="schedule"
				/>
				<div class="mb-8" />
			{/if}

			<h2 class="border-b pb-1 mb-2">
				<span class="mr-1">Schedule</span>
				<Tooltip>Schedules use CRON syntax. Seconds are mandatory.</Tooltip>
			</h2>
			<CronInput disabled={!can_write} bind:schedule bind:validCRON />

			<h2 class="border-b pb-1 mt-8 mb-2">Runnable</h2>
			{#if !edit}
				<p class="text-xs mb-1 text-gray-600">
					Pick a script or flow to be triggered by the schedule<Required required={true} />
				</p>
				<ScriptPicker
					disabled={initialScriptPath != '' || !can_write}
					initialPath={initialScriptPath}
					kind={Script.kind.SCRIPT}
					allowFlow={true}
					bind:itemKind
					bind:scriptPath={script_path}
				/>
			{:else}
				<Alert type="info" title="Runnable path cannot be edited">
					Once a schedule is created, the runnable path cannot be changed. However, when renaming a
					script or a flow, the runnable path will automatically update itself.
				</Alert>
				<div class="my-2" />
				<ScriptPicker
					disabled
					initialPath={script_path}
					scriptPath={script_path}
					allowFlow={true}
					{itemKind}
				/>
			{/if}
			<div class="mt-6">
				<span class="font-semibold text-gray-700">Arguments</span>

				{#if runnable}
					{#if runnable?.schema && runnable.schema.properties && Object.keys(runnable.schema.properties).length > 0}
						<SchemaForm disabled={!can_write} schema={runnable.schema} bind:isValid bind:args />
					{:else}
						<div class="text-xs texg-gray-700">
							This {is_flow ? 'flow' : 'script'} takes no argument
						</div>
					{/if}
				{:else}
					<div class="text-xs texg-gray-700 my-2">
						Pick a {is_flow ? 'flow' : 'script'} and fill its argument here
					</div>
				{/if}
			</div>
		</div>
	</DrawerContent>
</Drawer>
