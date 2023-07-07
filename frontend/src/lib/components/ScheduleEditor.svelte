<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import CronInput from '$lib/components/CronInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { FlowService, ScheduleService, Script, ScriptService, type Flow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, formatCron, sendUserToast } from '$lib/utils'
	import { faList, faSave } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'

	let initialPath = ''
	let edit = true
	let schedule: string = '0 0 12 * *'
	let timezone: string = Intl.DateTimeFormat().resolvedOptions().timeZone

	let itemKind: 'flow' | 'script' = 'script'
	let errorHandleritemKind: 'flow' | 'script' = 'script'
	let errorHandlerPath: string | undefined = undefined

	let script_path = ''
	let initialScriptPath = ''

	export function openEdit(ePath: string, isFlow: boolean) {
		is_flow = isFlow
		initialPath = ePath
		itemKind = is_flow ? 'flow' : 'script'
		if (path == ePath) {
			loadSchedule()
		} else {
			path = ePath
		}
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
		errorHandleritemKind = 'script'
		errorHandlerPath = undefined
		timezone = Intl.DateTimeFormat().resolvedOptions().timeZone
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

	// set isValid to true when a script/flow without any properties is selected
	$: runnable?.schema &&
		runnable.schema.properties &&
		Object.keys(runnable.schema.properties).length === 0 &&
		(isValid = true)

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
			timezone = s.timezone
			script_path = s.script_path ?? ''
			is_flow = s.is_flow
			if (s.on_failure) {
				console.log(s.on_failure)
				let splitted = s.on_failure.split('/')
				errorHandleritemKind = splitted[0] as 'flow' | 'script'
				errorHandlerPath = splitted.slice(1)?.join('/')
			} else {
				errorHandlerPath = undefined
				errorHandleritemKind = 'script'
			}
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
					timezone,
					args,
					on_failure: errorHandlerPath ? `${errorHandleritemKind}/${errorHandlerPath}` : undefined
				}
			})
			sendUserToast(`Schedule ${path} updated`)
		} else {
			await ScheduleService.createSchedule({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					schedule: formatCron(schedule),
					timezone,
					script_path,
					is_flow,
					args,
					enabled: true,
					on_failure: errorHandlerPath ? `${errorHandleritemKind}/${errorHandlerPath}` : undefined
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
		title={edit ? `Edit schedule ${initialPath}` : 'New schedule'}
		on:close={drawer.closeDrawer}
	>
		<svelte:fragment slot="actions">
			{#if edit}
				<div class="mr-8">
					<Button
						size="sm"
						variant="border"
						startIcon={{ icon: faList }}
						disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
						href={`/runs/${script_path}`}
					>
						View Runs
					</Button>
				</div>
				<div class="mr-8 center-center -mt-2">
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

			<div class="flex flex-row items-center mb-2 gap-1">
				<div class="text-xl font-extrabold">Schedule</div>
				<Tooltip>Schedules use CRON syntax. Seconds are mandatory.</Tooltip>
			</div>

			<CronInput disabled={!can_write} bind:schedule bind:timezone bind:validCRON />

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
			<h2 class="border-b pb-1 mt-8 mb-2">Error Handler</h2>

			<ScriptPicker
				disabled={initialScriptPath != '' || !can_write}
				initialPath={errorHandlerPath}
				kind={Script.kind.SCRIPT}
				allowFlow={true}
				bind:scriptPath={errorHandlerPath}
				bind:itemKind={errorHandleritemKind}
				canRefresh
			/>
			<div class="flex gap-20 items-start mt-3">
				<div class="text-gray-600 italic text-sm"
					>The following args will be passed to the error handler:
					<ul class="mt-1 ml-2">
						<li><b>path</b>: The path of the script or flow that errored</li>
						<li><b>schedule_path</b>: The path of the schedule</li>
						<li><b>error</b>: The error details</li>
					</ul>
				</div>
				<Button
					wrapperClasses="mt-6"
					href="/scripts/add?hub=hub%2F1087%2Fwindmill%2Fschedule_error_handler_template"
					target="_blank">Use template</Button
				>
			</div>
		</div>
	</DrawerContent>
</Drawer>
