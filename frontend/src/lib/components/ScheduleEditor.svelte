<script lang="ts">
	import { Alert, Button, Tab, Tabs } from '$lib/components/common'
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
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptySchema, emptyString, formatCron, sendUserToast } from '$lib/utils'
	import { faList, faSave } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import { inferArgs } from '$lib/infer'
	import type { Schema, SupportedLanguage } from '$lib/common'
	import Section from '$lib/components/Section.svelte'

	const slackErrorHandler = 'hub/2431/slack/schedule-error-handler-slack'
	const slackRecoveryHandler = 'hub/2430/slack/schedule-recovery-handler-slack'

	let initialPath = ''
	let edit = true
	let schedule: string = '0 0 12 * *'
	let timezone: string = Intl.DateTimeFormat().resolvedOptions().timeZone

	let itemKind: 'flow' | 'script' = 'script'
	let errorHandleritemKind: 'flow' | 'script' = 'script'
	let errorHandlerPath: string | undefined = undefined
	let recoveryHandlerPath: string | undefined = undefined
	let errorHandlerSelected: 'custom' | 'slack' = 'custom'
	let errorHandlerSchema: Schema | undefined = undefined
	let errorHandlerExtraArgs: Record<string, any> = {}
	let recoveryHandlerSelected: 'custom' | 'slack' = 'custom'
	let recoveryHandlerItemKind: 'flow' | 'script' = 'script'
	let recoveryHandlerSchema: Schema | undefined = undefined
	let recoveryHandlerExtraArgs: Record<string, any> = {}
	let failedTimes = 1
	let failedExact = false
	let recoveredTimes = 1

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
				let splitted = s.on_failure.split('/')
				errorHandleritemKind = splitted[0] as 'flow' | 'script'
				errorHandlerPath = splitted.slice(1)?.join('/')
				failedTimes = s.on_failure_times ?? 1
				failedExact = s.on_failure_exact ?? false
				errorHandlerExtraArgs = s.on_failure_extra_args ?? {}
				if (errorHandlerPath == slackErrorHandler) {
					errorHandlerSelected = 'slack'
				}
			} else {
				errorHandlerPath = undefined
				errorHandleritemKind = 'script'
			}
			if (s.on_recovery) {
				let splitted = s.on_recovery.split('/')
				recoveryHandlerItemKind = splitted[0] as 'flow' | 'script'
				recoveryHandlerPath = splitted.slice(1)?.join('/')
				recoveredTimes = s.on_recovery_times ?? 1
				recoveryHandlerExtraArgs = s.on_recovery_extra_args ?? {}
				if (recoveryHandlerPath == slackRecoveryHandler) {
					recoveryHandlerSelected = 'slack'
				}
			} else {
				recoveryHandlerPath = undefined
				recoveryHandlerItemKind = 'script'
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
					on_failure: errorHandlerPath ? `${errorHandleritemKind}/${errorHandlerPath}` : undefined,
					on_failure_times: failedTimes,
					on_failure_exact: failedExact,
					on_failure_extra_args: errorHandlerExtraArgs,
					on_recovery: recoveryHandlerPath
						? `${recoveryHandlerItemKind}/${recoveryHandlerPath}`
						: undefined,
					on_recovery_times: recoveredTimes,
					on_recovery_extra_args: recoveryHandlerExtraArgs
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
					on_failure: errorHandlerPath ? `${errorHandleritemKind}/${errorHandlerPath}` : undefined,
					on_failure_times: failedTimes,
					on_failure_exact: failedExact,
					on_failure_extra_args: errorHandlerExtraArgs,
					on_recovery: recoveryHandlerPath
						? `${recoveryHandlerItemKind}/${recoveryHandlerPath}`
						: undefined,
					on_recovery_times: recoveredTimes,
					on_recovery_extra_args: recoveryHandlerExtraArgs
				}
			})
			sendUserToast(`Schedule ${path} created`)
		}
		dispatch('update')
		drawer.closeDrawer()
	}

	async function loadHandlerScriptArgs(p: string, defaultArgs: string[] = []) {
		try {
			let schema: Schema | undefined = emptySchema()
			if (p.startsWith('hub/')) {
				const hubScript = await ScriptService.getHubScriptByPath({
					path: p
				})

				if (hubScript.schema?.properties) {
					schema = hubScript.schema
				} else {
					await inferArgs(hubScript.language as SupportedLanguage, hubScript.content ?? '', schema)
				}
			} else {
				const script = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: p })
				schema = script.schema as Schema
			}
			if (schema && schema.properties) {
				for (let key in schema.properties) {
					if (defaultArgs.includes(key)) {
						delete schema.properties[key]
					}
				}
				return schema
			}
		} catch (err) {
			sendUserToast(`Could not query handler schema: ${err}`, true)
		}
	}

	$: {
		if ($workspaceStore) {
			if (edit && path != '') {
				loadSchedule()
			}
		}
	}

	$: errorHandlerPath &&
		loadHandlerScriptArgs(errorHandlerPath, [
			'path',
			'is_flow',
			'schedule_path',
			'error',
			'failed_times',
			'started_at'
		]).then((schema) => (errorHandlerSchema = schema))
	$: recoveryHandlerPath &&
		loadHandlerScriptArgs(recoveryHandlerPath, [
			'path',
			'is_flow',
			'schedule_path',
			'error',
			'error_started_at',
			'success_times',
			'success_result',
			'success_started_at'
		]).then((schema) => (recoveryHandlerSchema = schema))

	$: errorHandlerSelected === 'slack' && (errorHandlerPath = slackErrorHandler)
	$: errorHandlerSelected === 'custom' && (errorHandlerPath = undefined)
	$: recoveryHandlerSelected === 'slack' && (recoveryHandlerPath = slackRecoveryHandler)
	$: recoveryHandlerSelected === 'custom' && (recoveryHandlerPath = undefined)

	let drawer: Drawer

	$: if (drawer) drawer.openDrawer()
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

		<div class="flex flex-col gap-8">
			<Section label="Schedule">
				<svelte:fragment slot="header">
					<Tooltip>Schedules use CRON syntax. Seconds are mandatory.</Tooltip>
				</svelte:fragment>
				<CronInput disabled={!can_write} bind:schedule bind:timezone bind:validCRON />
			</Section>
		</div>

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

			<h2 class="border-b pb-1 mt-8 mb-2">Runnable</h2>
			{#if !edit}
				<p class="text-xs mb-1 text-tertiary">
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
			<h2 class="border-b pb-1 mt-8 mb-2"
				>Error Handler <Tooltip>
					<div class="flex gap-20 items-start mt-3">
						<div class="text-tertiary text-sm"
							>The following args will be passed to the error handler:
							<ul class="mt-1 ml-2">
								<li><b>path</b>: The path of the script or flow that failed.</li>
								<li><b>is_flow</b>: Whether the runnable is a flow.</li>
								<li><b>schedule_path</b>: The path of the schedule.</li>
								<li><b>error</b>: The error details.</li>
								<li
									><b>failed_times</b>: Minimum number of times the schedule failed before calling
									the error handler.</li
								>
								<li><b>started_at</b>: The start datetime of the latest job that failed.</li>
							</ul>
						</div>
					</div>
				</Tooltip></h2
			>

			<div>
				<Tabs bind:selected={errorHandlerSelected} class="mt-2 mb-4">
					<Tab value="custom">Custom</Tab>
					<Tab value="slack">Slack</Tab>
				</Tabs>
			</div>

			{#if errorHandlerSelected === 'custom'}
				<div class="flex flex-row mb-2">
					<ScriptPicker
						disabled={!can_write}
						initialPath={errorHandlerPath}
						kind={Script.kind.SCRIPT}
						allowFlow={true}
						bind:scriptPath={errorHandlerPath}
						bind:itemKind={errorHandleritemKind}
						allowRefresh
					/>

					{#if errorHandlerPath === undefined}
						<Button
							btnClasses="ml-4 mt-2"
							color="dark"
							size="xs"
							href="/scripts/add?hub=hub%2F2420%2Fwindmill%2Fschedule_error_handler_template"
							target="_blank">Create from template</Button
						>
					{/if}
				</div>
			{:else if errorHandlerSelected === 'slack'}
				<Alert type="info" title="Slack schedule error handler"
					>You will receive a notification on the selected slack channel.
				</Alert>
			{/if}

			<div class="flex flex-row items-center justify-between">
				<div class="flex flex-row items-center mt-4 font-semibold text-sm gap-2">
					<p
						>{#if !$enterpriseLicense}<span class="text-normal text-2xs">(ee only)</span>{/if} Triggered
						when schedule failed</p
					>
					<select class="!w-14" bind:value={failedExact} disabled={!$enterpriseLicense}>
						<option value={false}>&gt;=</option>
						<option value={true}>==</option>
					</select>
					<input
						type="number"
						class="!w-14 text-center"
						bind:value={failedTimes}
						disabled={!$enterpriseLicense}
						min="1"
					/>
					<p>time{failedTimes > 1 ? 's in a row' : ''}</p>
				</div>
			</div>
			{#if errorHandlerPath}
				<p class="font-semibold text-sm mt-4 mb-2"
					>{errorHandlerSelected !== 'custom' ? 'Configuration' : 'Extra arguments'}</p
				>
				<SchemaForm
					disabled={!can_write}
					schema={errorHandlerSchema}
					bind:args={errorHandlerExtraArgs}
					shouldHideNoInputs
					class="text-xs"
				/>
				{#if errorHandlerSchema && errorHandlerSchema.properties && Object.keys(errorHandlerSchema.properties).length === 0}
					<div class="text-xs texg-gray-700">This error handler takes no extra arguments</div>
				{/if}
			{/if}

			<h2 class="border-b pb-1 mt-8 mb-2"
				>Recovery Handler {#if !$enterpriseLicense}<span class="text-normal text-2xs"
						>(ee only)</span
					>{/if}
				<Tooltip
					><div class="text-tertiary text-sm"
						>The following args will be passed to the recovery handler:
						<ul class="mt-1 ml-2">
							<li><b>path</b>: The path of the script or flow that recovered.</li>
							<li><b>is_flow</b>: Whether the runnable is a flow.</li>
							<li><b>schedule_path</b>: The path of the schedule.</li>
							<li><b>error</b>: The error of the last job that errored</li>
							<li><b>error_started_at</b>: The start datetime of the last job that errored</li>
							<li
								><b>success_times</b>: The number of times the schedule succeeded before calling the
								recovery handler.</li
							>
							<li><b>success_result</b>: The result of the latest successful job</li>
							<li><b>success_started_at</b>: The start datetime of the latest successful job</li>
						</ul>
					</div></Tooltip
				></h2
			>

			<Tabs bind:selected={recoveryHandlerSelected} class="mt-2 mb-4">
				<Tab value="custom">Custom</Tab>
				<Tab value="slack">Slack</Tab>
			</Tabs>

			{#if recoveryHandlerSelected === 'custom'}
				<div class="flex flex-row mb-2">
					<ScriptPicker
						disabled={!can_write || !$enterpriseLicense}
						initialPath={recoveryHandlerPath}
						kind={Script.kind.SCRIPT}
						allowFlow={true}
						bind:scriptPath={recoveryHandlerPath}
						bind:itemKind={recoveryHandlerItemKind}
						allowRefresh
					/>

					{#if recoveryHandlerPath === undefined}
						<Button
							btnClasses="ml-4 mt-2"
							color="dark"
							size="xs"
							href="/scripts/add?hub=hub%2F2421%2Fwindmill%2Fschedule_recovery_handler_template"
							target="_blank">Create from template</Button
						>
					{/if}
				</div>
			{:else if recoveryHandlerSelected === 'slack'}
				<Alert type="info" title="Slack schedule recovery handler"
					>You will receive a notification on the selected slack channel.
				</Alert>
			{/if}

			<div class="flex flex-row items-center justify-between">
				<div class="flex flex-row items-center mt-5 font-semibold text-sm">
					<p>Triggered when schedule recovered</p>
					<input type="number" class="!w-14 mx-2 text-center" bind:value={recoveredTimes} min="1" />
					<p>time{recoveredTimes > 1 ? 's in a row' : ''}</p>
				</div>
			</div>

			{#if recoveryHandlerPath}
				<p class="font-semibold text-sm mt-4 mb-2"
					>{recoveryHandlerSelected === 'custom' ? 'Extra arguments' : 'Configuration'}</p
				>
				<SchemaForm
					disabled={!can_write}
					schema={recoveryHandlerSchema}
					bind:args={recoveryHandlerExtraArgs}
					shouldHideNoInputs
					class="text-xs"
				/>
				{#if recoveryHandlerSchema && recoveryHandlerSchema.properties && Object.keys(recoveryHandlerSchema.properties).length === 0}
					<div class="text-xs texg-gray-700">This recovery handler takes no extra arguments</div>
				{/if}
			{/if}
		</div>
	</DrawerContent>
</Drawer>
