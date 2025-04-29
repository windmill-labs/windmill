<script lang="ts">
	import { Alert, Badge, Button, Tab, Tabs } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import CronInput from '$lib/components/CronInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import ErrorOrRecoveryHandler from '$lib/components/ErrorOrRecoveryHandler.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import {
		FlowService,
		ScheduleService,
		type Script,
		ScriptService,
		type Flow,
		SettingService,
		type Retry,
		type Schedule
	} from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, formatCron, sendUserToast, cronV1toV2 } from '$lib/utils'
	import { base } from '$lib/base'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { List, Loader2, Save, AlertTriangle } from 'lucide-svelte'
	import autosize from '$lib/autosize'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'
	import FlowRetries from '$lib/components/flows/content/FlowRetries.svelte'
	import Label from '$lib/components/Label.svelte'
	import WorkerTagPicker from '$lib/components/WorkerTagPicker.svelte'
	import { runScheduleNow } from '../scheduled/utils'

	let optionTabSelected: 'error_handler' | 'recovery_handler' | 'success_handler' | 'retries' =
		'error_handler'

	let is_flow: boolean = false
	let initialPath = ''
	let edit = true
	let schedule: string = '0 0 12 * *'
	let cronVersion: string = 'v2'
	let isLatestCron = true
	let initialCronVersion: string = 'v2'
	let initialSchedule: string
	let timezone: string = Intl.DateTimeFormat().resolvedOptions().timeZone
	let paused_until: string | undefined = undefined

	let itemKind: 'flow' | 'script' = 'script'
	let errorHandleritemKind: 'flow' | 'script' = 'script'
	let wsErrorHandlerMuted: boolean = false
	let errorHandlerPath: string | undefined = undefined
	let errorHandlerCustomInitialPath: string | undefined = undefined
	let errorHandlerSelected: 'custom' | 'slack' | 'teams' = 'slack'
	let errorHandlerExtraArgs: Record<string, any> = {}
	let recoveryHandlerPath: string | undefined = undefined
	let recoveryHandlerCustomInitialPath: string | undefined = undefined
	let recoveryHandlerSelected: 'custom' | 'slack' | 'teams' = 'slack'
	let recoveryHandlerItemKind: 'flow' | 'script' = 'script'
	let recoveryHandlerExtraArgs: Record<string, any> = {}
	let successHandlerPath: string | undefined = undefined
	let successHandlerCustomInitialPath: string | undefined = undefined
	let successHandlerSelected: 'custom' | 'slack' | 'teams' = 'slack'
	let successHandlerItemKind: 'flow' | 'script' = 'script'
	let successHandlerExtraArgs: Record<string, any> = {}
	let failedTimes = 1
	let failedExact = false
	let recoveredTimes = 1
	let duplicate = false
	let retry: Retry | undefined = undefined

	let script_path = ''
	let initialScriptPath = ''

	let runnable: Script | Flow | undefined
	let args: Record<string, any> = {}

	let loading = false

	let drawerLoading = true
	export function openEdit(ePath: string, isFlow: boolean) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = isFlow
			initialPath = ePath
			itemKind = is_flow ? 'flow' : 'script'
			if (path == ePath) {
				loadSchedule()
			} else {
				path = ePath
			}
			edit = true
		} finally {
			drawerLoading = false
		}
	}

	async function setScheduleHandler(s?: Schedule) {
		if (s) {
			if (s.on_failure) {
				let splitted = s.on_failure.split('/')
				errorHandleritemKind = splitted[0] as 'flow' | 'script'
				errorHandlerPath = splitted.slice(1)?.join('/')
				errorHandlerCustomInitialPath = errorHandlerPath
				failedTimes = s.on_failure_times ?? 1
				failedExact = s.on_failure_exact ?? false
				errorHandlerExtraArgs = s.on_failure_extra_args ?? {}
				errorHandlerSelected = getHandlerType('error', errorHandlerPath)
			} else {
				errorHandlerPath = undefined
				errorHandleritemKind = 'script'
				errorHandlerCustomInitialPath = undefined
				errorHandlerExtraArgs = {}
				failedExact = false
				failedTimes = 1
				errorHandlerSelected = 'slack'
			}
			if (s.on_recovery) {
				let splitted = s.on_recovery.split('/')
				recoveryHandlerItemKind = splitted[0] as 'flow' | 'script'
				recoveryHandlerPath = splitted.slice(1)?.join('/')
				recoveryHandlerCustomInitialPath = recoveryHandlerPath
				recoveredTimes = s.on_recovery_times ?? 1
				recoveryHandlerExtraArgs = s.on_recovery_extra_args ?? {}
				recoveryHandlerSelected = getHandlerType('recovery', recoveryHandlerPath)
			} else {
				recoveryHandlerPath = undefined
				recoveryHandlerItemKind = 'script'
				recoveryHandlerCustomInitialPath = undefined
				recoveredTimes = 1
				recoveryHandlerSelected = 'slack'
				recoveryHandlerExtraArgs = {}
			}
			if (s.on_success) {
				let splitted = s.on_success.split('/')
				successHandlerItemKind = splitted[0] as 'flow' | 'script'
				successHandlerPath = splitted.slice(1)?.join('/')
				successHandlerCustomInitialPath = successHandlerPath
				successHandlerExtraArgs = s.on_success_extra_args ?? {}
				successHandlerSelected = getHandlerType('success', successHandlerPath)
			} else {
				successHandlerPath = undefined
				successHandlerItemKind = 'script'
				successHandlerCustomInitialPath = undefined
				successHandlerSelected = 'slack'
				successHandlerExtraArgs = {}
			}
		}
		else {
			let defaultErrorHandlerMaybe = undefined
			let defaultRecoveryHandlerMaybe = undefined
			let defaultSuccessHandlerMaybe = undefined
			if ($workspaceStore) {
				defaultErrorHandlerMaybe = (await SettingService.getGlobal({
					key: 'default_error_handler_' + $workspaceStore!
				})) as any
				defaultRecoveryHandlerMaybe = (await SettingService.getGlobal({
					key: 'default_recovery_handler_' + $workspaceStore!
				})) as any
				defaultSuccessHandlerMaybe = (await SettingService.getGlobal({
					key: 'default_success_handler_' + $workspaceStore!
				})) as any
			}

			if (defaultErrorHandlerMaybe !== undefined && defaultErrorHandlerMaybe !== null) {
				wsErrorHandlerMuted = defaultErrorHandlerMaybe['wsErrorHandlerMuted']
				let splitted = (defaultErrorHandlerMaybe['errorHandlerPath'] as string).split('/')
				errorHandleritemKind = splitted[0] as 'flow' | 'script'
				errorHandlerPath = splitted.slice(1)?.join('/')
				errorHandlerExtraArgs = defaultErrorHandlerMaybe['errorHandlerExtraArgs']
				errorHandlerCustomInitialPath = errorHandlerPath
				errorHandlerSelected = getHandlerType('error', errorHandlerPath)
				failedTimes = defaultErrorHandlerMaybe['failedTimes']
				failedExact = defaultErrorHandlerMaybe['failedExact']
			} else {
				wsErrorHandlerMuted = false
				errorHandlerPath = undefined
				errorHandleritemKind = 'script'
				errorHandlerExtraArgs = {}
				errorHandlerCustomInitialPath = undefined
				errorHandlerSelected = 'slack'
				failedTimes = 1
				failedExact = false
			}
			if (defaultRecoveryHandlerMaybe !== undefined && defaultRecoveryHandlerMaybe !== null) {
				let splitted = (defaultRecoveryHandlerMaybe['recoveryHandlerPath'] as string).split('/')
				recoveryHandlerItemKind = splitted[0] as 'flow' | 'script'
				recoveryHandlerPath = splitted.slice(1)?.join('/')
				recoveryHandlerExtraArgs = defaultRecoveryHandlerMaybe['recoveryHandlerExtraArgs']
				recoveryHandlerCustomInitialPath = recoveryHandlerPath
				recoveryHandlerSelected = getHandlerType('recovery', recoveryHandlerPath)
				recoveredTimes = defaultRecoveryHandlerMaybe['recoveredTimes']
			} else {
				recoveryHandlerPath = undefined
				recoveryHandlerItemKind = 'script'
				recoveryHandlerExtraArgs = {}
				recoveryHandlerCustomInitialPath = undefined
				recoveryHandlerSelected = 'slack'
				recoveredTimes = 1
			}
			if (defaultSuccessHandlerMaybe !== undefined && defaultSuccessHandlerMaybe !== null) {
				let splitted = (defaultSuccessHandlerMaybe['successHandlerPath'] as string).split('/')
				successHandlerItemKind = splitted[0] as 'flow' | 'script'
				successHandlerPath = splitted.slice(1)?.join('/')
				successHandlerExtraArgs = defaultSuccessHandlerMaybe['successHandlerExtraArgs']
				successHandlerCustomInitialPath = successHandlerPath
				successHandlerSelected = getHandlerType('success', successHandlerPath)
				recoveredTimes = defaultSuccessHandlerMaybe['recoveredTimes']
			} else {
				successHandlerPath = undefined
				successHandlerItemKind = 'script'
				successHandlerExtraArgs = {}
				successHandlerCustomInitialPath = undefined
				successHandlerSelected = 'slack'
			}
		}
	}

	export async function openNew(
		nis_flow: boolean,
		initial_script_path?: string,
		schedule_path?: string
	) {
		drawerLoading = true
		try {
			let s: Schedule | undefined
			if (schedule_path) {
				s = await ScheduleService.getSchedule({
					workspace: $workspaceStore!,
					path: schedule_path
				})
				duplicate = true
			}
			drawer?.openDrawer()
			runnable = undefined
			is_flow = s?.is_flow ?? nis_flow
			edit = false
			itemKind = is_flow ? 'flow' : 'script'
			initialScriptPath = initial_script_path ?? ''
			path = duplicate === true ? '' : initialScriptPath

			initialPath = path
			cronVersion = s?.cron_version ?? 'v2'
			initialCronVersion = cronVersion
			isLatestCron = cronVersion == 'v2'
			schedule = s?.schedule ?? '0 0 12 * *'
			initialSchedule = schedule
			timezone = s?.timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone
			paused_until = s?.paused_until
			showPauseUntil = paused_until !== undefined
			summary = s?.summary ?? ''
			description = s?.description ?? ''
			script_path = s?.script_path ?? initialScriptPath
			args = s?.args ?? {}
			tag = s?.tag

			await loadScript(script_path)

			no_flow_overlap = s?.no_flow_overlap ?? false
			wsErrorHandlerMuted = s?.ws_error_handler_muted ?? false
			retry = s?.retry

			setScheduleHandler(s)
		} finally {
			drawerLoading = false
		}
	}

	async function resetRetries() {
		if (itemKind === 'flow') {
			retry = undefined
		}
	}

	$: (is_flow = itemKind == 'flow') && resetRetries()

	let isValid = true

	let path: string = ''
	let enabled: boolean = false
	let pathError = ''
	let summary = ''
	let description = ''
	let no_flow_overlap = false
	let tag: string | undefined = undefined

	let validCRON = true
	$: allowSchedule = isValid && validCRON && script_path != ''

	// set isValid to true when a script/flow without any properties is selected
	$: runnable?.schema &&
		runnable.schema.properties &&
		Object.keys(runnable.schema.properties).length === 0 &&
		(isValid = true)

	const dispatch = createEventDispatcher()

	async function loadScript(p: string | undefined): Promise<void> {
		if (p) {
			runnable = undefined
			try {
				if (is_flow) {
					runnable = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path: p })
				} else {
					runnable = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: p })
				}
			} catch (err) {}
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

	async function saveAsDefaultSuccessHandler(overrideExisting: boolean) {
		if (!$enterpriseLicense) {
			sendUserToast(`Setting default success handler is an enterprise edition feature`, true)
			return
		}
		if ($workspaceStore) {
			await ScheduleService.setDefaultErrorOrRecoveryHandler({
				workspace: $workspaceStore!,
				requestBody: {
					handler_type: 'success',
					override_existing: overrideExisting,
					path:
						successHandlerPath === undefined
							? undefined
							: `${successHandlerItemKind}/${successHandlerPath}`,
					extra_args: successHandlerExtraArgs,
					number_of_occurence: recoveredTimes
				}
			})
			if (successHandlerPath !== undefined) {
				sendUserToast(`Default success handler saved to ${successHandlerPath}`, false)
			} else {
				sendUserToast(`Default success handler reset`, false)
			}
		}
	}

	let can_write = true
	async function loadSchedule(): Promise<void> {
		loading = true
		try {
			const s = await ScheduleService.getSchedule({
				workspace: $workspaceStore!,
				path: initialPath
			})
			is_flow = s.is_flow
			cronVersion = s.cron_version ?? 'v2'
			initialCronVersion = cronVersion
			isLatestCron = cronVersion == 'v2'
			enabled = s.enabled
			schedule = s.schedule
			initialSchedule = schedule
			timezone = s.timezone
			paused_until = s.paused_until
			showPauseUntil = paused_until !== undefined
			summary = s.summary ?? ''
			description = s.description ?? ''
			script_path = s.script_path ?? ''
			args = s.args ?? {}
			can_write = canWrite(s.path, s.extra_perms, $userStore)
			tag = s.tag

			await loadScript(script_path)

			no_flow_overlap = s.no_flow_overlap ?? false
			wsErrorHandlerMuted = s.ws_error_handler_muted ?? false
			retry = s.retry
			setScheduleHandler(s)
		} catch (err) {
			sendUserToast(`Could not load schedule: ${err}`, true)
		}
		loading = false
	}

	async function scheduleScript(): Promise<void> {
		if (errorHandlerPath !== undefined && isSlackHandler('error', errorHandlerPath)) {
			errorHandlerExtraArgs['slack'] = '$res:f/slack_bot/bot_token'
		} else {
			errorHandlerExtraArgs['slack'] = undefined
		}
		if (recoveryHandlerPath !== undefined && isSlackHandler('recovery', recoveryHandlerPath)) {
			recoveryHandlerExtraArgs['slack'] = '$res:f/slack_bot/bot_token'
		} else {
			recoveryHandlerExtraArgs['slack'] = undefined
		}
		if (successHandlerPath !== undefined && isSlackHandler('success', successHandlerPath)) {
			successHandlerExtraArgs['slack'] = '$res:f/slack_bot/bot_token'
		} else {
			successHandlerExtraArgs['slack'] = undefined
		}
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
					on_failure_extra_args: errorHandlerPath ? errorHandlerExtraArgs : undefined,
					on_recovery: recoveryHandlerPath
						? `${recoveryHandlerItemKind}/${recoveryHandlerPath}`
						: undefined,
					on_recovery_times: recoveredTimes,
					on_recovery_extra_args: recoveryHandlerPath ? recoveryHandlerExtraArgs : {},
					on_success: successHandlerPath
						? `${successHandlerItemKind}/${successHandlerPath}`
						: undefined,
					on_success_extra_args: successHandlerPath ? successHandlerExtraArgs : {},
					ws_error_handler_muted: wsErrorHandlerMuted,
					retry: retry,
					summary: summary != '' ? summary : undefined,
					description: description,
					no_flow_overlap: no_flow_overlap,
					tag: tag,
					paused_until: paused_until,
					cron_version: cronVersion
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
					on_success: successHandlerPath
						? `${successHandlerItemKind}/${successHandlerPath}`
						: undefined,
					on_success_extra_args: successHandlerPath ? successHandlerExtraArgs : {},
					ws_error_handler_muted: wsErrorHandlerMuted,
					retry: retry,
					summary: summary != '' ? summary : undefined,
					description: description,
					no_flow_overlap: no_flow_overlap,
					tag: tag,
					paused_until: paused_until,
					cron_version: cronVersion
				}
			})
			sendUserToast(`Schedule ${path} created`)
		}
		dispatch('update')
		drawer.closeDrawer()
	}

	function getHandlerType(
		isHandler: 'error' | 'recovery' | 'success',
		scriptPath: string
	): 'custom' | 'slack' | 'teams' {
		const handlerMap = {
			error: {
				teams: '/workspace-or-schedule-error-handler-teams',
				slack: '/workspace-or-schedule-error-handler-slack'
			},
			recovery: {
				teams: '/schedule-recovery-handler-teams',
				slack: '/schedule-recovery-handler-slack'
			},
			success: {
				teams: '/schedule-success-handler-teams',
				slack: '/schedule-success-handler-slack'
			}
		}

		for (const [type, suffix] of Object.entries(handlerMap[isHandler])) {
			if (scriptPath.startsWith('hub/') && scriptPath.endsWith(suffix)) {
				return type as 'custom' | 'slack' | 'teams'
			}
		}
		return 'custom'
	}

	function isSlackHandler(isSlackHandler: 'error' | 'recovery' | 'success', scriptPath: string) {
		if (isSlackHandler == 'error') {
			return (
				scriptPath.startsWith('hub/') &&
				scriptPath.endsWith('/workspace-or-schedule-error-handler-slack')
			)
		} else if (isSlackHandler == 'recovery') {
			return (
				scriptPath.startsWith('hub/') && scriptPath.endsWith('/schedule-recovery-handler-slack')
			)
		} else {
			return scriptPath.startsWith('hub/') && scriptPath.endsWith('/schedule-success-handler-slack')
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

	let pathC: Path
	let dirtyPath = false

	let showPauseUntil = false
	$: !showPauseUntil && (paused_until = undefined)

	function onVersionChange() {
		cronVersion = isLatestCron ? 'v2' : 'v1'
		if (cronVersion === 'v2' && initialCronVersion === 'v1') {
			// switches day-of-week from v1 -> v2
			schedule = cronV1toV2(schedule)
		} else if (
			cronVersion === 'v1' &&
			initialCronVersion === 'v1' &&
			schedule !== initialSchedule
		) {
			// revert back to original
			schedule = initialSchedule
		}
	}
</script>

<Drawer size="900px" bind:this={drawer}>
	<DrawerContent
		title={edit
			? can_write
				? `Edit schedule ${initialPath}`
				: `View schedule ${initialPath}`
			: 'New schedule'}
		on:close={drawer.closeDrawer}
	>
		<svelte:fragment slot="actions">
			{#if !drawerLoading}
				{#if edit}
					<div class="mr-8 flex flex-row gap-3">
						<Button
							size="sm"
							variant="border"
							startIcon={{ icon: List }}
							disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
							href={`${base}/runs/${script_path}?show_schedules=true&show_future_jobs=true`}
						>
							View runs
						</Button>
						<Button
							size="sm"
							variant="border"
							disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
							on:click={() => {
								runScheduleNow(script_path, path, is_flow, $workspaceStore!)
							}}
						>
							Run now
						</Button>
					</div>
					{#if can_write}
						<div class="mr-8 center-center -mt-1">
							<Toggle
								disabled={!can_write}
								checked={enabled}
								options={{ right: 'Enabled' }}
								on:change={async (e) => {
									await ScheduleService.setScheduleEnabled({
										path: initialPath,
										workspace: $workspaceStore ?? '',
										requestBody: { enabled: e.detail }
									})
									dispatch('update')
									sendUserToast(`${e.detail ? 'enabled' : 'disabled'} schedule ${initialPath}`)
								}}
							/>
						</div>
					{/if}
				{/if}
				<Button
					startIcon={{ icon: Save }}
					disabled={!allowSchedule ||
						pathError != '' ||
						emptyString(script_path) ||
						(errorHandlerSelected == 'slack' &&
							!emptyString(errorHandlerPath) &&
							emptyString(errorHandlerExtraArgs['channel'])) ||
						!can_write}
					on:click={scheduleScript}
				>
					{edit ? 'Save' : 'Schedule'}
				</Button>
			{/if}
		</svelte:fragment>
		{#if drawerLoading}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="flex flex-col gap-12">
				<div class="flex flex-col gap-4">
					<div>
						<h2 class="text-base font-semibold mb-2">Metadata</h2>
						<Label label="Summary">
							<!-- svelte-ignore a11y-autofocus -->
							<input
								autofocus
								type="text"
								placeholder="Short summary to be displayed when listed"
								class="text-sm w-full"
								bind:value={summary}
								disabled={!can_write}
								on:keyup={() => {
									if (!edit && summary?.length > 0 && !dirtyPath) {
										pathC?.setName(
											summary
												.toLowerCase()
												.replace(/[^a-z0-9_]/g, '_')
												.replace(/-+/g, '_')
												.replace(/^-|-$/g, '')
										)
									}
								}}
							/>
						</Label>
					</div>
					<Label label="Path">
						{#if !edit}
							<Path
								bind:dirty={dirtyPath}
								bind:this={pathC}
								checkInitialPathExistence
								bind:error={pathError}
								bind:path
								{initialPath}
								namePlaceholder="schedule"
								kind="schedule"
							/>
						{:else}
							<div class="flex justify-start w-full">
								<Badge
									color="gray"
									class="center-center !bg-surface-secondary !text-tertiary  !h-[24px] rounded-r-none border"
								>
									Schedule path (not editable)
								</Badge>
								<input
									type="text"
									readonly
									value={path}
									size={path?.length || 50}
									class="font-mono !text-xs grow shrink overflow-x-auto !h-[24px] !py-0 !border-l-0 !rounded-l-none"
									on:focus={({ currentTarget }) => {
										currentTarget.select()
									}}
								/>
								<!-- <span class="font-mono text-sm break-all">{path}</span> -->
							</div>
						{/if}
					</Label>

					<Label label="Description">
						<textarea
							rows="4"
							use:autosize
							bind:value={description}
							placeholder="What this schedule does and how to use it"
						></textarea>
					</Label>
				</div>

				<Section label="Schedule">
					<svelte:fragment slot="header">
						{#if cronVersion === 'v1'}
							<Tooltip>Schedules use CRON syntax. Seconds are mandatory.</Tooltip>
						{:else}
							<Tooltip
								>Schedules use <a
									href="https://www.windmill.dev/docs/core_concepts/scheduling#cron-syntax"
									>extended CRON syntax</a
								>.</Tooltip
							>
						{/if}
					</svelte:fragment>
					{#if initialCronVersion !== 'v2'}
						<div class="flex flex-row">
							<AlertTriangle color="orange" class="mr-2" size={16} />
							<Toggle
								options={{
									right: 'enable latest Cron syntax',
									rightTooltip:
										'The latest Cron syntax is more flexible and allows for more complex schedules. See the documentation for more information.',
									rightDocumentationLink:
										'https://www.windmill.dev/docs/core_concepts/scheduling#cron-syntax'
								}}
								size="xs"
								bind:checked={isLatestCron}
								on:change={onVersionChange}
								disabled={!can_write}
							/>
						</div>
					{/if}
					<CronInput
						disabled={!can_write}
						bind:schedule
						bind:timezone
						bind:validCRON
						bind:cronVersion
					/>
					<Toggle
						options={{
							right: 'Pause schedule until...',
							rightTooltip:
								'Pausing the schedule will program the next job to run as if the schedule starts at the time the pause is lifted, instead of now.'
						}}
						bind:checked={showPauseUntil}
						size="xs"
						disabled={!can_write}
					/>
					{#if showPauseUntil}
						<DateTimeInput bind:value={paused_until} />
					{/if}
				</Section>
				<Section label="Runnable">
					{#if !edit}
						<p class="text-xs mb-1 text-tertiary">
							Pick a script or flow to be triggered by the schedule<Required required={true} />
						</p>
						<ScriptPicker
							disabled={(initialScriptPath != '' && !duplicate) || !can_write}
							initialPath={initialScriptPath}
							kinds={['script']}
							allowFlow={true}
							allowRefresh={can_write}
							bind:itemKind
							bind:scriptPath={script_path}
							on:select={(e) => {
								loadScript(e.detail.path)
							}}
						/>
					{:else}
						<Alert type="info" title="Runnable path cannot be edited" collapsible>
							Once a schedule is created, the runnable path cannot be changed. However, when
							renaming a script or a flow, the runnable path will automatically update itself.
						</Alert>
						<div class="my-2"></div>
						<ScriptPicker
							disabled
							initialPath={script_path}
							scriptPath={script_path}
							allowFlow={true}
							{itemKind}
							allowView={script_path != '' && !!runnable}
							allowEdit={script_path != '' && !!runnable && !$userStore?.operator}
						/>
					{/if}
					{#if itemKind == 'flow'}
						<Toggle
							options={{ right: 'no overlap of flows' }}
							bind:checked={no_flow_overlap}
							class="mt-2"
						/>
					{/if}
					{#if itemKind == 'script'}
						<div class="flex gap-2 items-center mt-2">
							<Toggle options={{ right: 'no overlap' }} checked={true} disabled /><Tooltip
								>Currently, overlapping scripts' executions is not supported. The next execution
								will be scheduled only after the previous iteration has completed.</Tooltip
							>
						</div>
					{/if}
					<div class="mt-6">
						{#if !loading}
							{#if runnable}
								{#if runnable?.schema && runnable.schema.properties && Object.keys(runnable.schema.properties).length > 0}
									{#await import('$lib/components/SchemaForm.svelte')}
										<Loader2 class="animate-spin" />
									{:then Module}
										<Module.default
											showReset
											disabled={!can_write}
											schema={runnable.schema}
											bind:isValid
											bind:args
										/>
									{/await}
								{:else}
									<div class="text-xs texg-gray-700">
										This {is_flow ? 'flow' : 'script'} takes no argument
									</div>
								{/if}
							{:else if script_path != ''}
								<div class="text-xs texg-gray-700 my-2">
									You cannot see the the {is_flow ? 'flow' : 'script'} input form as you do not have
									access to it.
								</div>
							{:else}
								<div class="text-xs texg-gray-700 my-2">
									Pick a {is_flow ? 'flow' : 'script'} and fill its argument here
								</div>
							{/if}
						{:else}
							<Loader2 class="animate-spin" />
						{/if}
					</div>
				</Section>

				<div class="flex flex-col gap-2">
					{#if !loading}
						<Tabs bind:selected={optionTabSelected}>
							<Tab value="error_handler">Error Handler</Tab>
							<Tab value="recovery_handler">Recovery Handler</Tab>
							<Tab value="success_handler">Success Handler</Tab>
							{#if itemKind === 'script'}
								<Tab value="retries">Retries</Tab>
								<Tab value="tag">Custom tag</Tab>
							{/if}
						</Tabs>
						<div class="pt-0.5"></div>
						{#if optionTabSelected === 'error_handler'}
							<Section label="Error handler">
								<svelte:fragment slot="header">
									<div class="flex flex-row gap-2">
										{#if !$enterpriseLicense}<span class="text-normal text-2xs">(ee only)</span
											>{/if}
									</div>
								</svelte:fragment>
								<svelte:fragment slot="action">
									<div class="flex flex-row items-center gap-1 text-2xs text-tertiary">
										defaults
										<Dropdown
											disabled={!can_write}
											items={[
												{
													displayName: `Override future schedules only`,
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
								<div class="flex flex-row py-2">
									<Toggle
										size="xs"
										disabled={!can_write || !$enterpriseLicense}
										bind:checked={wsErrorHandlerMuted}
										options={{ right: 'Mute workspace error handler for this schedule' }}
									/>
								</div>

								<ErrorOrRecoveryHandler
									isEditable={can_write}
									errorOrRecovery="error"
									showScriptHelpText={true}
									bind:handlerSelected={errorHandlerSelected}
									bind:handlerPath={errorHandlerPath}
									customInitialScriptPath={errorHandlerCustomInitialPath}
									toggleText="Alert channel on error"
									customScriptTemplate="/scripts/add?hub=hub%2F9081%2Fwindmill%2Fschedule_error_handler_template"
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
															><b>failed_times</b>: Minimum number of times the schedule failed
															before calling the error handler.</li
														>
														<li
															><b>started_at</b>: The start datetime of the latest job that failed.</li
														>
													</ul>
												</div>
											</div>
										</Tooltip>
									</svelte:fragment>
								</ErrorOrRecoveryHandler>
								<div class="flex flex-row items-center justify-between">
									<div class="flex flex-row items-center mt-4 font-semibold text-sm gap-2">
										<p class={emptyString(errorHandlerPath) ? 'text-tertiary' : ''}>
											Triggered when schedule failed</p
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
											class="!w-14 text-center {emptyString(errorHandlerPath)
												? 'text-tertiary'
												: ''}"
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
						{:else if optionTabSelected === 'recovery_handler'}
							{@const disabled = !can_write || emptyString($enterpriseLicense)}
							<Section label="Recovery handler">
								<svelte:fragment slot="header">
									<div class="flex flex-row gap-2">
										{#if !$enterpriseLicense}<span class="text-normal text-2xs">(ee only)</span
											>{/if}
									</div>
								</svelte:fragment>
								<svelte:fragment slot="action">
									<div class="flex flex-row items-center text-tertiary text-2xs gap-2">
										defaults
										<Dropdown
											{disabled}
											items={[
												{
													displayName: `Override future schedules only`,
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
									isEditable={!disabled}
									errorOrRecovery="recovery"
									bind:handlerSelected={recoveryHandlerSelected}
									bind:handlerPath={recoveryHandlerPath}
									customInitialScriptPath={recoveryHandlerCustomInitialPath}
									toggleText="Alert channel when error recovered"
									customScriptTemplate="/scripts/add?hub=hub%2F9082%2Fwindmill%2Fschedule_recovery_handler_template"
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
														<li
															><b>error_started_at</b>: The start datetime of the last job that
															errored</li
														>
														<li
															><b>success_times</b>: The number of times the schedule succeeded
															before calling the recovery handler.</li
														>
														<li><b>success_result</b>: The result of the latest successful job</li>
														<li
															><b>success_started_at</b>: The start datetime of the latest
															successful job</li
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
											{disabled}
										/>
										<p>time{recoveredTimes > 1 ? 's in a row' : ''}</p>
									</div>
								</div>
							</Section>
						{:else if optionTabSelected === 'success_handler'}
							{@const disabled = !can_write || emptyString($enterpriseLicense)}
							<Section label="Success handler">
								<svelte:fragment slot="header">
									<div class="flex flex-row gap-2">
										{#if !$enterpriseLicense}<span class="text-normal text-2xs">(ee only)</span
											>{/if}
									</div>
								</svelte:fragment>
								<svelte:fragment slot="action">
									<div class="flex flex-row items-center text-tertiary text-2xs gap-2">
										defaults
										<Dropdown
											{disabled}
											items={[
												{
													displayName: `Override future schedules only`,
													action: () => saveAsDefaultSuccessHandler(false)
												},
												{
													displayName: 'Override all existing',
													type: 'delete',
													action: () => saveAsDefaultSuccessHandler(true)
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
									isEditable={!disabled}
									errorOrRecovery="success"
									bind:handlerSelected={successHandlerSelected}
									bind:handlerPath={successHandlerPath}
									customInitialScriptPath={successHandlerCustomInitialPath}
									toggleText="Alert channel when successful"
									customScriptTemplate="/scripts/add?hub=hub%2F9071%2Fwindmill%2Fschedule_success_handler_template"
									bind:customHandlerKind={successHandlerItemKind}
									bind:handlerExtraArgs={successHandlerExtraArgs}
								>
									<svelte:fragment slot="custom-tab-tooltip">
										<Tooltip>
											<div class="flex gap-20 items-start mt-3">
												<div class=" text-sm"
													>The following args will be passed to the success handler:
													<ul class="mt-1 ml-2">
														<li><b>path</b>: The path of the script or flow that succeeded.</li>
														<li><b>is_flow</b>: Whether the runnable is a flow.</li>
														<li><b>schedule_path</b>: The path of the schedule.</li>
														<li><b>success_result</b>: The result of the successful job</li>
														<li
															><b>success_started_at</b>: The start datetime of the successful job</li
														>
													</ul>
												</div>
											</div>
										</Tooltip>
									</svelte:fragment>
								</ErrorOrRecoveryHandler>
							</Section>
						{:else if optionTabSelected === 'retries'}
							{@const disabled = !can_write || emptyString($enterpriseLicense)}
							<Section label="Retries">
								<svelte:fragment slot="header">
									<div class="flex flex-row gap-2">
										{#if !$enterpriseLicense}<span class="text-normal text-2xs">(ee only)</span
											>{/if}
									</div>
									<Tooltip>
										If defined, upon error this schedule will be retried with a delay and a maximum
										number of attempts as defined below.
										<br />
										This is only available for individual script. For flows, retries can be set on each
										flow step in the flow editor.
									</Tooltip>
								</svelte:fragment>
								<FlowRetries
									bind:flowModuleRetry={retry}
									disabled={itemKind !== 'script' || disabled}
								/>
							</Section>
						{:else if optionTabSelected === 'tag'}
							<Section
								label="Custom script tag"
								tooltip="When set, the script tag will be overridden by this tag"
							>
								<WorkerTagPicker bind:tag popupPlacement="top-end" disabled={!can_write} />
							</Section>
						{/if}
					{:else}
						<Loader2 class="animate-spin" />
					{/if}
				</div>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
