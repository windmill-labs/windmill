<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import CronInput from '$lib/components/CronInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import ErrorOrRecoveryHandler from '$lib/components/ErrorOrRecoveryHandler.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import {
		FlowService,
		ScheduleService,
		Script,
		ScriptService,
		type Flow,
		SettingService
	} from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, formatCron, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { List, Save } from 'lucide-svelte'

	let initialPath = ''
	let edit = true
	let schedule: string = '0 0 12 * *'
	let timezone: string = Intl.DateTimeFormat().resolvedOptions().timeZone

	let itemKind: 'flow' | 'script' = 'script'
	let errorHandleritemKind: 'flow' | 'script' = 'script'
	let wsErrorHandlerMuted: boolean = false
	let errorHandlerPath: string | undefined = undefined
	let errorHandlerCustomInitialPath: string | undefined = undefined
	let errorHandlerSelected: 'custom' | 'slack' = 'slack'
	let errorHandlerExtraArgs: Record<string, any> = {}
	let recoveryHandlerPath: string | undefined = undefined
	let recoveryHandlerCustomInitialPath: string | undefined = undefined
	let recoveryHandlerSelected: 'custom' | 'slack' = 'slack'
	let recoveryHandlerItemKind: 'flow' | 'script' = 'script'
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

	export async function openNew(is_flow: boolean, initial_script_path?: string) {
		let defaultErrorHandlerMaybe = undefined
		let defaultRecoveryHandlerMaybe = undefined
		if ($workspaceStore) {
			defaultErrorHandlerMaybe = await SettingService.getGlobal({
				key: 'default_error_handler_' + $workspaceStore!
			})
			defaultRecoveryHandlerMaybe = await SettingService.getGlobal({
				key: 'default_recovery_handler_' + $workspaceStore!
			})
		}

		edit = false
		itemKind = is_flow ? 'flow' : 'script'
		initialScriptPath = initial_script_path ?? ''
		path = initialScriptPath
		initialPath = initialScriptPath
		script_path = initialScriptPath
		if (defaultErrorHandlerMaybe !== undefined && defaultErrorHandlerMaybe !== null) {
			console.log(defaultErrorHandlerMaybe)
			wsErrorHandlerMuted = defaultErrorHandlerMaybe['wsErrorHandlerMuted']
			let splitted = (defaultErrorHandlerMaybe['errorHandlerPath'] as string).split('/')
			errorHandleritemKind = splitted[0] as 'flow' | 'script'
			errorHandlerPath = splitted.slice(1)?.join('/')
			errorHandlerExtraArgs = defaultErrorHandlerMaybe['errorHandlerExtraArgs']
			errorHandlerCustomInitialPath = errorHandlerPath
			errorHandlerSelected = isSlackHandler('error', errorHandlerPath) ? 'slack' : 'custom'
			failedTimes = defaultErrorHandlerMaybe['failedTimes']
			failedExact = defaultErrorHandlerMaybe['failedExact']
		} else {
			wsErrorHandlerMuted = false
			errorHandlerPath = undefined
			errorHandleritemKind = 'script'
			errorHandlerExtraArgs = {}
			errorHandlerCustomInitialPath = undefined
			errorHandlerSelected = $enterpriseLicense ? 'slack' : 'custom'
		}
		if (defaultRecoveryHandlerMaybe !== undefined && defaultRecoveryHandlerMaybe !== null) {
			let splitted = (defaultRecoveryHandlerMaybe['recoveryHandlerPath'] as string).split('/')
			recoveryHandlerItemKind = splitted[0] as 'flow' | 'script'
			recoveryHandlerPath = splitted.slice(1)?.join('/')
			recoveryHandlerExtraArgs = defaultRecoveryHandlerMaybe['recoveryHandlerExtraArgs']
			recoveryHandlerCustomInitialPath = recoveryHandlerPath
			recoveryHandlerSelected = isSlackHandler('recovery', recoveryHandlerPath) ? 'slack' : 'custom'
			recoveredTimes = defaultRecoveryHandlerMaybe['recoveredTimes']
		} else {
			recoveryHandlerPath = undefined
			recoveryHandlerItemKind = 'script'
			recoveryHandlerExtraArgs = {}
			recoveryHandlerCustomInitialPath = undefined
			recoveryHandlerSelected = $enterpriseLicense ? 'slack' : 'custom'
		}
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

	async function saveAsDefaultErrorHandler(overrideExisting: boolean) {
		if (!$enterpriseLicense) {
			sendUserToast(`Setting default error handler is an enterprise edition feature`, true)
			return
		}
		if ($workspaceStore) {
			await ScheduleService.setDefaultErrorOrRecoveryHandler({
				workspace: $workspaceStore!,
				requestBody: {
					handler_type: 'error',
					override_existing: overrideExisting,
					path:
						errorHandlerPath == undefined
							? undefined
							: `${errorHandleritemKind}/${errorHandlerPath}`,
					extra_args: errorHandlerExtraArgs,
					number_of_occurence: failedTimes,
					number_of_occurence_exact: failedExact,
					workspace_handler_muted: wsErrorHandlerMuted
				}
			})
			if (errorHandlerPath !== undefined) {
				sendUserToast(`Default error handler saved to ${errorHandlerPath}`, false)
			} else {
				sendUserToast(`Default error handler reset`, false)
			}
		}
	}

	async function saveAsDefaultRecoveryHandler(overrideExisting: boolean) {
		if (!$enterpriseLicense) {
			sendUserToast(`Setting default recovery handler is an enterprise edition feature`, true)
			return
		}
		if ($workspaceStore) {
			await ScheduleService.setDefaultErrorOrRecoveryHandler({
				workspace: $workspaceStore!,
				requestBody: {
					handler_type: 'recovery',
					override_existing: overrideExisting,
					path:
						recoveryHandlerPath === undefined
							? undefined
							: `${recoveryHandlerItemKind}/${recoveryHandlerPath}`,
					extra_args: recoveryHandlerExtraArgs,
					number_of_occurence: recoveredTimes
				}
			})
			if (recoveryHandlerPath !== undefined) {
				sendUserToast(`Default recovery handler saved to ${recoveryHandlerPath}`, false)
			} else {
				sendUserToast(`Default recovery handler reset`, false)
			}
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
			wsErrorHandlerMuted = s.ws_error_handler_muted ?? false
			if (s.on_failure) {
				let splitted = s.on_failure.split('/')
				errorHandleritemKind = splitted[0] as 'flow' | 'script'
				errorHandlerPath = splitted.slice(1)?.join('/')
				errorHandlerCustomInitialPath = errorHandlerPath
				failedTimes = s.on_failure_times ?? 1
				failedExact = s.on_failure_exact ?? false
				errorHandlerExtraArgs = s.on_failure_extra_args ?? {}
				errorHandlerSelected = isSlackHandler('error', errorHandlerPath) ? 'slack' : 'custom'
			} else {
				errorHandlerPath = undefined
				errorHandleritemKind = 'script'
			}
			if (s.on_recovery) {
				let splitted = s.on_recovery.split('/')
				recoveryHandlerItemKind = splitted[0] as 'flow' | 'script'
				recoveryHandlerPath = splitted.slice(1)?.join('/')
				recoveryHandlerCustomInitialPath = recoveryHandlerPath
				recoveredTimes = s.on_recovery_times ?? 1
				recoveryHandlerExtraArgs = s.on_recovery_extra_args ?? {}
				recoveryHandlerSelected = isSlackHandler('recovery', recoveryHandlerPath)
					? 'slack'
					: 'custom'
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
			if (errorHandlerPath !== undefined && isSlackHandler('error', errorHandlerPath)) {
				errorHandlerExtraArgs['slack'] = '$res:f/slack_bot/bot_token'
			}
			if (recoveryHandlerPath !== undefined && isSlackHandler('recovery', recoveryHandlerPath)) {
				recoveryHandlerExtraArgs['slack'] = '$res:f/slack_bot/bot_token'
			}
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
					on_failure_extra_args: errorHandlerPath ? errorHandlerExtraArgs : undefined,
					on_recovery: recoveryHandlerPath
						? `${recoveryHandlerItemKind}/${recoveryHandlerPath}`
						: undefined,
					on_recovery_times: recoveredTimes,
					on_recovery_extra_args: recoveryHandlerPath ? recoveryHandlerExtraArgs : {},
					ws_error_handler_muted: wsErrorHandlerMuted
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
					on_failure_extra_args: errorHandlerPath ? errorHandlerExtraArgs : undefined,
					on_recovery: recoveryHandlerPath
						? `${recoveryHandlerItemKind}/${recoveryHandlerPath}`
						: undefined,
					on_recovery_times: recoveredTimes,
					on_recovery_extra_args: recoveryHandlerPath ? recoveryHandlerExtraArgs : {},
					ws_error_handler_muted: wsErrorHandlerMuted
				}
			})
			sendUserToast(`Schedule ${path} created`)
		}
		dispatch('update')
		drawer.closeDrawer()
	}

	function isSlackHandler(isSlackHandler: 'error' | 'recovery', scriptPath: string) {
		if (isSlackHandler == 'error') {
			return (
				scriptPath.startsWith('hub/') &&
				scriptPath.endsWith('/workspace-or-schedule-error-handler-slack')
			)
		} else {
			return (
				scriptPath.startsWith('hub/') && scriptPath.endsWith('/schedule-recovery-handler-slack')
			)
		}
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
						startIcon={{ icon: List }}
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
				startIcon={{ icon: Save }}
				disabled={!allowSchedule ||
					pathError != '' ||
					emptyString(script_path) ||
					(errorHandlerSelected == 'slack' &&
						!emptyString(errorHandlerPath) &&
						emptyString(errorHandlerExtraArgs['channel']))}
				on:click={scheduleScript}
			>
				{edit ? 'Save' : 'Schedule'}
			</Button>
		</svelte:fragment>

		<div class="flex flex-col gap-12">
			{#if !edit}
				<Section label="Metadata">
					<Path
						checkInitialPathExistence
						bind:error={pathError}
						bind:path
						{initialPath}
						namePlaceholder="schedule"
						kind="schedule"
					/>
				</Section>
			{/if}
			<Section label="Schedule">
				<svelte:fragment slot="header">
					<Tooltip>Schedules use CRON syntax. Seconds are mandatory.</Tooltip>
				</svelte:fragment>
				<CronInput disabled={!can_write} bind:schedule bind:timezone bind:validCRON />
			</Section>
			<Section label="Runnable">
				{#if !edit}
					<p class="text-xs mb-1 text-tertiary">
						Pick a script or flow to be triggered by the schedule<Required required={true} />
					</p>
					<ScriptPicker
						disabled={initialScriptPath != '' || !can_write}
						initialPath={initialScriptPath}
						kinds={[Script.kind.SCRIPT]}
						allowFlow={true}
						bind:itemKind
						bind:scriptPath={script_path}
					/>
				{:else}
					<Alert type="info" title="Runnable path cannot be edited">
						Once a schedule is created, the runnable path cannot be changed. However, when renaming
						a script or a flow, the runnable path will automatically update itself.
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
			</Section>
			<Section label="Error handler">
				<svelte:fragment slot="action">
					<div class="flex flex-row items-center gap-2">
						<Dropdown
							items={[
								{
									displayName: `Future schedules only`,
									action: () => saveAsDefaultErrorHandler(false)
								},
								{
									displayName: 'Override all existing',
									type: 'delete',
									action: () => saveAsDefaultErrorHandler(true)
								}
							]}
						>
							<svelte:fragment>
								<Save size={12} class="mr-1" />
								Set as default
							</svelte:fragment>
						</Dropdown>
					</div>
				</svelte:fragment>
				<div class="flex flex-row">
					<Toggle
						disabled={!can_write}
						bind:checked={wsErrorHandlerMuted}
						options={{ right: 'Mute workspace error handler for this schedule' }}
					/>
				</div>
				<ErrorOrRecoveryHandler
					isEditable={can_write}
					errorOrRecovery="error"
					handlersOnlyForEe={['slack']}
					showScriptHelpText={true}
					bind:handlerSelected={errorHandlerSelected}
					bind:handlerPath={errorHandlerPath}
					customInitialScriptPath={errorHandlerCustomInitialPath}
					slackToggleText="Alert channel on error"
					customScriptTemplate="/scripts/add?hub=hub%2F2420%2Fwindmill%2Fschedule_error_handler_template"
					bind:customHandlerKind={errorHandleritemKind}
					bind:handlerExtraArgs={errorHandlerExtraArgs}
				>
					<svelte:fragment slot="custom-tab-tooltip">
						<Tooltip>
							<div class="flex gap-20 items-start mt-3">
								<div class="text-sm"
									>The following args will be passed to the error handler:
									<ul class="mt-1 ml-2">
										<li><b>path</b>: The path of the script or flow that failed.</li>
										<li><b>is_flow</b>: Whether the runnable is a flow.</li>
										<li><b>schedule_path</b>: The path of the schedule.</li>
										<li><b>error</b>: The error details.</li>
										<li
											><b>failed_times</b>: Minimum number of times the schedule failed before
											calling the error handler.</li
										>
										<li><b>started_at</b>: The start datetime of the latest job that failed.</li>
									</ul>
								</div>
							</div>
						</Tooltip>
					</svelte:fragment>
				</ErrorOrRecoveryHandler>

				<div class="flex flex-row items-center justify-between">
					<div class="flex flex-row items-center mt-4 font-semibold text-sm gap-2">
						<p class={emptyString(errorHandlerPath) ? 'text-tertiary' : ''}
							>{#if !$enterpriseLicense}<span class="text-normal text-2xs">(ee only)</span>{/if} Triggered
							when schedule failed</p
						>
						<select
							class="!w-14"
							bind:value={failedExact}
							disabled={!$enterpriseLicense || emptyString(errorHandlerPath)}
						>
							<option value={false}>&gt;=</option>
							<option value={true}>==</option>
						</select>
						<input
							type="number"
							class="!w-14 text-center {emptyString(errorHandlerPath) ? 'text-tertiary' : ''}"
							bind:value={failedTimes}
							disabled={!$enterpriseLicense}
							min="1"
						/>
						<p class={emptyString(errorHandlerPath) ? 'text-tertiary' : ''}
							>time{failedTimes > 1 ? 's in a row' : ''}</p
						>
					</div>
				</div>
			</Section>
			<Section label="Recovery handler">
				<svelte:fragment slot="header">
					<div class="flex flex-row gap-2">
						{#if !$enterpriseLicense}<span class="text-normal text-2xs">(ee only)</span>{/if}
					</div>
				</svelte:fragment>
				<svelte:fragment slot="action">
					<div class="flex flex-row items-center gap-2">
						<Dropdown
							items={[
								{
									displayName: `Future schedules only`,
									action: () => saveAsDefaultRecoveryHandler(false)
								},
								{
									displayName: 'Override all existing',
									type: 'delete',
									action: () => saveAsDefaultRecoveryHandler(true)
								}
							]}
						>
							<svelte:fragment>
								<Save size={12} class="mr-1" />
								Set as default
							</svelte:fragment>
						</Dropdown>
					</div>
				</svelte:fragment>

				<ErrorOrRecoveryHandler
					isEditable={can_write && !emptyString($enterpriseLicense)}
					errorOrRecovery="recovery"
					handlersOnlyForEe={[]}
					bind:handlerSelected={recoveryHandlerSelected}
					bind:handlerPath={recoveryHandlerPath}
					customInitialScriptPath={recoveryHandlerCustomInitialPath}
					slackToggleText="Alert channel when error recovered"
					customScriptTemplate="/scripts/add?hub=hub%2F2421%2Fwindmill%2Fschedule_recovery_handler_template"
					bind:customHandlerKind={recoveryHandlerItemKind}
					bind:handlerExtraArgs={recoveryHandlerExtraArgs}
				>
					<svelte:fragment slot="custom-tab-tooltip">
						<Tooltip>
							<div class="flex gap-20 items-start mt-3">
								<div class=" text-sm"
									>The following args will be passed to the recovery handler:
									<ul class="mt-1 ml-2">
										<li><b>path</b>: The path of the script or flow that recovered.</li>
										<li><b>is_flow</b>: Whether the runnable is a flow.</li>
										<li><b>schedule_path</b>: The path of the schedule.</li>
										<li><b>error</b>: The error of the last job that errored</li>
										<li><b>error_started_at</b>: The start datetime of the last job that errored</li
										>
										<li
											><b>success_times</b>: The number of times the schedule succeeded before
											calling the recovery handler.</li
										>
										<li><b>success_result</b>: The result of the latest successful job</li>
										<li
											><b>success_started_at</b>: The start datetime of the latest successful job</li
										>
									</ul>
								</div>
							</div>
						</Tooltip>
					</svelte:fragment>
				</ErrorOrRecoveryHandler>

				<div class="flex flex-row items-center justify-between">
					<div
						class="flex flex-row items-center mt-5 font-semibold text-sm {emptyString(
							recoveryHandlerPath
						)
							? 'text-tertiary'
							: ''}"
					>
						<p>Triggered when schedule recovered</p>
						<input
							type="number"
							class="!w-14 mx-2 text-center"
							bind:value={recoveredTimes}
							min="1"
						/>
						<p>time{recoveredTimes > 1 ? 's in a row' : ''}</p>
					</div>
				</div>
			</Section>
		</div>
	</DrawerContent>
</Drawer>
