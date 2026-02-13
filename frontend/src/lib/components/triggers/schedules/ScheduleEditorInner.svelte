<script lang="ts">
	import { Alert, Badge, Button, ButtonType, Tab, Tabs } from '$lib/components/common'
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
		type Schedule,
		type ErrorHandler
	} from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, formatCron, sendUserToast, cronV1toV2 } from '$lib/utils'
	import { base } from '$lib/base'
	import Section from '$lib/components/Section.svelte'
	import { List, Loader2, Save, AlertTriangle } from 'lucide-svelte'
	import autosize from '$lib/autosize'
	import TriggerEditorToolbar from '$lib/components/triggers/TriggerEditorToolbar.svelte'
	import { saveScheduleFromCfg } from '$lib/components/flows/scheduleUtils'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'
	import FlowRetries from '$lib/components/flows/content/FlowRetries.svelte'
	import Label from '$lib/components/Label.svelte'
	import WorkerTagPicker from '$lib/components/WorkerTagPicker.svelte'
	import { runScheduleNow } from '../scheduled/utils'
	import { handleConfigChange } from '../utils'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { twMerge } from 'tailwind-merge'

	let {
		useDrawer = true,
		hideTarget = false,
		docDescription = undefined,
		allowDraft = false,
		draftSchema = undefined,
		customLabel = undefined,
		isDeployed = false,
		onUpdate = undefined,
		onConfigChange = undefined,
		onDelete = undefined,
		onReset = undefined,
		trigger = undefined
	} = $props()

	let optionTabSelected:
		| 'error_handler'
		| 'recovery_handler'
		| 'success_handler'
		| 'retries'
		| 'dynamic_skip' = $state('error_handler')
	let initialPath = $state('')
	let edit = $state(true)
	let schedule: string = $state('0 0 12 * *')
	let cronVersion: string = $state('v2')
	let isLatestCron = $state(true)
	let initialCronVersion: string = $state('v2')
	let initialSchedule: string
	let timezone: string = $state(Intl.DateTimeFormat().resolvedOptions().timeZone)
	let paused_until: string | undefined = $state(undefined)
	let itemKind: 'flow' | 'script' = $state('script')
	let is_flow: boolean = $derived.by(() => itemKind === 'flow')
	let errorHandleritemKind: 'flow' | 'script' = $state('script')
	let wsErrorHandlerMuted: boolean = $state(false)
	let errorHandlerPath: string | undefined = $state(undefined)
	let errorHandlerSelected: ErrorHandler = $state('slack')
	let errorHandlerExtraArgs: Record<string, any> = $state({})
	let recoveryHandlerPath: string | undefined = $state(undefined)
	let recoveryHandlerSelected: ErrorHandler = $state('slack')
	let recoveryHandlerItemKind: 'flow' | 'script' = $state('script')
	let recoveryHandlerExtraArgs: Record<string, any> = $state({})
	let successHandlerPath: string | undefined = $state(undefined)
	let successHandlerSelected: ErrorHandler = $state('slack')
	let successHandlerItemKind: 'flow' | 'script' = $state('script')
	let successHandlerExtraArgs: Record<string, any> = $state({})
	let failedTimes = $state(1)
	let failedExact = $state(false)
	let recoveredTimes = $state(1)
	let retry: Retry | undefined = $state(undefined)
	let dynamicSkipPath: string | undefined = $state(undefined)
	let script_path = $state('')
	let initialScriptPath = $state('')
	let runnable: Script | Flow | undefined = $state()
	let args: Record<string, any> = $state({})
	let loading = $state(false)
	let drawerLoading = $state(true)
	let showLoading = $state(false)
	let initialConfig: Record<string, any> | undefined = undefined
	let extraPerms: Record<string, boolean> = $state({})
	let can_write = $state(true)
	let initNewPath = $state(false)
	let path: string = $state('')
	let enabled: boolean = $state(false)
	let pathError = $state('')
	let summary = $state('')
	let description = $state('')
	let no_flow_overlap = $state(false)
	let tag: string | undefined = $state(undefined)
	let validCRON = $state(true)
	let isValid = $state(true)
	let allowSchedule = $derived(isValid && validCRON && script_path != '')
	let deploymentLoading = $state(false)

	const saveDisabled = $derived(
		!allowSchedule ||
			pathError != '' ||
			emptyString(script_path) ||
			(errorHandlerSelected == 'slack' &&
				!emptyString(errorHandlerPath) &&
				emptyString(errorHandlerExtraArgs['channel'])) ||
			!can_write
	)
	const scheduleCfg = $derived.by(getScheduleCfg)

	export async function openEdit(ePath: string, isFlow: boolean, defaultCfg?: Record<string, any>) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			path = defaultCfg?.path ?? ePath
			await loadSchedule(defaultCfg)
			edit = true
		} finally {
			if (!defaultCfg) {
				initialConfig = structuredClone($state.snapshot(getScheduleCfg()))
			}
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	async function setScheduleHandler(s?: Schedule) {
		if (s) {
			if (s.on_failure) {
				let splitted = s.on_failure.split('/')
				errorHandleritemKind = splitted[0] as 'flow' | 'script'
				errorHandlerPath = splitted.slice(1)?.join('/')
				failedTimes = s.on_failure_times ?? 1
				failedExact = s.on_failure_exact ?? false
				errorHandlerExtraArgs = s.on_failure_extra_args ?? {}
				errorHandlerSelected = getHandlerType('error', errorHandlerPath)
			} else {
				errorHandlerPath = undefined
				errorHandleritemKind = 'script'
				errorHandlerExtraArgs = {}
				failedExact = false
				failedTimes = 1
				errorHandlerSelected = 'slack'
			}
			if (s.on_recovery) {
				let splitted = s.on_recovery.split('/')
				recoveryHandlerItemKind = splitted[0] as 'flow' | 'script'
				recoveryHandlerPath = splitted.slice(1)?.join('/')
				recoveredTimes = s.on_recovery_times ?? 1
				recoveryHandlerExtraArgs = s.on_recovery_extra_args ?? {}
				recoveryHandlerSelected = getHandlerType('recovery', recoveryHandlerPath)
			} else {
				recoveryHandlerPath = undefined
				recoveryHandlerItemKind = 'script'
				recoveredTimes = 1
				recoveryHandlerSelected = 'slack'
				recoveryHandlerExtraArgs = {}
			}
			if (s.on_success) {
				let splitted = s.on_success.split('/')
				successHandlerItemKind = splitted[0] as 'flow' | 'script'
				successHandlerPath = splitted.slice(1)?.join('/')
				successHandlerExtraArgs = s.on_success_extra_args ?? {}
				successHandlerSelected = getHandlerType('success', successHandlerPath)
			} else {
				successHandlerPath = undefined
				successHandlerItemKind = 'script'
				successHandlerSelected = 'slack'
				successHandlerExtraArgs = {}
			}
		} else {
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
				errorHandlerSelected = getHandlerType('error', errorHandlerPath)
				failedTimes = defaultErrorHandlerMaybe['failedTimes']
				failedExact = defaultErrorHandlerMaybe['failedExact']
			} else {
				wsErrorHandlerMuted = false
				errorHandlerPath = undefined
				errorHandleritemKind = 'script'
				errorHandlerExtraArgs = {}
				errorHandlerSelected = 'slack'
				failedTimes = 1
				failedExact = false
			}
			if (defaultRecoveryHandlerMaybe !== undefined && defaultRecoveryHandlerMaybe !== null) {
				let splitted = (defaultRecoveryHandlerMaybe['recoveryHandlerPath'] as string).split('/')
				recoveryHandlerItemKind = splitted[0] as 'flow' | 'script'
				recoveryHandlerPath = splitted.slice(1)?.join('/')
				recoveryHandlerExtraArgs = defaultRecoveryHandlerMaybe['recoveryHandlerExtraArgs']
				recoveryHandlerSelected = getHandlerType('recovery', recoveryHandlerPath)
				recoveredTimes = defaultRecoveryHandlerMaybe['recoveredTimes']
			} else {
				recoveryHandlerPath = undefined
				recoveryHandlerItemKind = 'script'
				recoveryHandlerExtraArgs = {}
				recoveryHandlerSelected = 'slack'
				recoveredTimes = 1
			}
			if (defaultSuccessHandlerMaybe !== undefined && defaultSuccessHandlerMaybe !== null) {
				let splitted = (defaultSuccessHandlerMaybe['successHandlerPath'] as string).split('/')
				successHandlerItemKind = splitted[0] as 'flow' | 'script'
				successHandlerPath = splitted.slice(1)?.join('/')
				successHandlerExtraArgs = defaultSuccessHandlerMaybe['successHandlerExtraArgs']
				successHandlerSelected = getHandlerType('success', successHandlerPath)
				recoveredTimes = defaultSuccessHandlerMaybe['recoveredTimes']
			} else {
				successHandlerPath = undefined
				successHandlerItemKind = 'script'
				successHandlerExtraArgs = {}
				successHandlerSelected = 'slack'
			}
		}
	}

	export async function openNew(
		nis_flow: boolean,
		initial_script_path?: string,
		defaultValues?: Schedule,
		schedule_path?: string
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		try {
			let s: Schedule | undefined
			if (schedule_path) {
				s = await ScheduleService.getSchedule({
					workspace: $workspaceStore!,
					path: schedule_path
				})
				initNewPath = true
			} else if (defaultValues) {
				s = defaultValues
			}
			drawer?.openDrawer()
			runnable = undefined
			edit = false
			itemKind = (s?.is_flow ?? nis_flow) ? 'flow' : 'script'
			initialScriptPath = initial_script_path ?? ''
			path = initNewPath
				? ''
				: (defaultValues?.path ?? (trigger?.isPrimary ? initialScriptPath : ''))
			initialPath = path
			cronVersion = s?.cron_version ?? 'v2'
			initialCronVersion = cronVersion
			isLatestCron = cronVersion == 'v2'
			schedule = s?.schedule ?? '0 0 12 * *'
			initialSchedule = schedule
			timezone = s?.timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone
			paused_until = s?.paused_until ?? undefined
			showPauseUntil = paused_until !== undefined
			summary = s?.summary ?? ''
			description = s?.description ?? ''
			script_path = s?.script_path ?? initialScriptPath
			args = s?.args ?? {}
			tag = s?.tag ?? undefined

			await loadScript(script_path)

			no_flow_overlap = s?.no_flow_overlap ?? false
			wsErrorHandlerMuted = s?.ws_error_handler_muted ?? false
			retry = s?.retry ?? undefined

			await setScheduleHandler(s)
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	// set isValid to true when a script/flow without any properties is selected
	$effect(() => {
		setDefaultValid(draftSchema ?? runnable?.schema)
	})

	function setDefaultValid(schema: Record<string, any> | undefined) {
		if (!isValid) {
			let isEmpty = schema?.properties == undefined || Object.keys(schema.properties).length === 0
			if (isEmpty) {
				isValid = true
			}
		}
	}

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

	async function loadSchedule(defaultCfg?: Record<string, any>): Promise<void> {
		if (!defaultCfg) {
			try {
				const s = await ScheduleService.getSchedule({
					workspace: $workspaceStore!,
					path: initialPath
				})
				await loadScheduleCfg(s)
			} catch (err) {
				sendUserToast(`Could not load schedule: ${err}`, true)
			}
		} else {
			await loadScheduleCfg(defaultCfg)
		}
	}

	async function loadScheduleCfg(cfg: Record<string, any>): Promise<void> {
		loading = true

		cronVersion = cfg.cron_version ?? 'v2'
		initialCronVersion = cronVersion
		isLatestCron = cronVersion == 'v2'
		enabled = cfg.enabled
		schedule = cfg.schedule
		initialSchedule = schedule
		timezone = cfg.timezone
		paused_until = cfg.paused_until
		showPauseUntil = paused_until !== undefined
		summary = cfg.summary ?? ''
		description = cfg.description ?? ''
		script_path = cfg.script_path ?? ''
		await loadScript(script_path)

		itemKind = cfg.is_flow ? 'flow' : 'script'
		no_flow_overlap = cfg.no_flow_overlap ?? false
		wsErrorHandlerMuted = cfg.ws_error_handler_muted ?? false
		retry = cfg.retry
		if (cfg.on_failure) {
			let splitted = cfg.on_failure.split('/')
			errorHandleritemKind = splitted[0] as 'flow' | 'script'
			errorHandlerPath = splitted.slice(1)?.join('/')
			failedTimes = cfg.on_failure_times ?? 1
			failedExact = cfg.on_failure_exact ?? false
			errorHandlerExtraArgs = cfg.on_failure_extra_args ?? {}
			errorHandlerSelected = getHandlerType('error', errorHandlerPath ?? '')
		} else {
			errorHandlerPath = undefined
			errorHandleritemKind = 'script'
			errorHandlerExtraArgs = {}
			failedExact = false
			failedTimes = 1
			errorHandlerSelected = 'slack'
		}
		if (cfg.on_recovery) {
			let splitted = cfg.on_recovery.split('/')
			recoveryHandlerItemKind = splitted[0] as 'flow' | 'script'
			recoveryHandlerPath = splitted.slice(1)?.join('/')
			recoveredTimes = cfg.on_recovery_times ?? 1
			recoveryHandlerExtraArgs = cfg.on_recovery_extra_args ?? {}
			recoveryHandlerSelected = getHandlerType('recovery', recoveryHandlerPath ?? '')
		} else {
			recoveryHandlerPath = undefined
			recoveryHandlerItemKind = 'script'
			recoveredTimes = 1
			recoveryHandlerSelected = 'slack'
			recoveryHandlerExtraArgs = {}
		}
		if (cfg.on_success) {
			let splitted = cfg.on_success.split('/')
			successHandlerItemKind = splitted[0] as 'flow' | 'script'
			successHandlerPath = splitted.slice(1)?.join('/')
			successHandlerExtraArgs = cfg.on_success_extra_args ?? {}
			successHandlerSelected = getHandlerType('success', successHandlerPath ?? '')
		} else {
			successHandlerPath = undefined
			successHandlerItemKind = 'script'
			successHandlerSelected = 'slack'
			successHandlerExtraArgs = {}
		}
		dynamicSkipPath = cfg.dynamic_skip
		args = cfg.args ?? {}
		extraPerms = cfg.extra_perms ?? {}
		can_write = canWrite(cfg.path, cfg.extra_perms, $userStore)
		tag = cfg.tag

		loading = false
	}

	async function scheduleScript(): Promise<void> {
		const scheduleCfg = getScheduleCfg()
		deploymentLoading = true
		const isSaved = await saveScheduleFromCfg(scheduleCfg, edit, $workspaceStore!)
		if (isSaved) {
			onUpdate?.(scheduleCfg.path)
			drawer?.closeDrawer()
		}
		deploymentLoading = false
	}

	function getHandlerType(
		isHandler: 'error' | 'recovery' | 'success',
		scriptPath: string
	): ErrorHandler {
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
				return type as ErrorHandler
			}
		}
		return 'custom'
	}

	let drawer: Drawer | undefined = $state()

	let pathC: Path | undefined = $state()
	let dirtyPath = $state(false)

	let showPauseUntil = $state(false)
	$effect(() => {
		!showPauseUntil && (paused_until = undefined)
	})

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

	function getScheduleCfg(): Record<string, any> {
		return {
			path: path,
			schedule: formatCron(schedule),
			timezone: timezone,
			script_path: script_path,
			is_flow: is_flow,
			args: args,
			enabled: enabled,
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
			cron_version: cronVersion,
			extra_perms: extraPerms,
			dynamic_skip: dynamicSkipPath
		}
	}

	async function handleToggleEnabled(nEnabled: boolean) {
		enabled = nEnabled
		if (!trigger?.draftConfig) {
			await ScheduleService.setScheduleEnabled({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { enabled: nEnabled }
			})
			sendUserToast(`${nEnabled ? 'enabled' : 'disabled'} schedule ${initialPath}`)
			onUpdate?.(initialPath)
		}
	}

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(scheduleCfg, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})
</script>

<!-- {JSON.stringify({ allowSchedule, path: script_path, validCRON, isValid })} -->
{#snippet saveButton()}
	{#if !drawerLoading}
		<TriggerEditorToolbar
			{trigger}
			permissions={drawerLoading || !can_write ? 'none' : 'create'}
			{saveDisabled}
			mode={enabled ? 'enabled' : 'disabled'}
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			onUpdate={scheduleScript}
			{onReset}
			{onDelete}
			onToggleMode={(mode) => handleToggleEnabled(mode === 'enabled')}
			{isDeployed}
			disableSuspendedMode
		>
			{#snippet extra()}
				{#if !drawerLoading && edit}
					<div class="mr-12 flex flex-row gap-3">
						<Button
							size="sm"
							variant="default"
							startIcon={{ icon: List }}
							disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
							href={`${base}/runs/${script_path}?show_schedules=true&show_future_jobs=true`}
						>
							View runs
						</Button>
						<Button
							size="sm"
							variant="default"
							disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
							on:click={() => {
								runScheduleNow(script_path, path, is_flow, $workspaceStore!)
							}}
						>
							Run now
						</Button>
					</div>
				{/if}
			{/snippet}
		</TriggerEditorToolbar>
	{/if}
{/snippet}

{#snippet content()}
	{#if drawerLoading}
		{#if showLoading}
			<Loader2 class="animate-spin" />
		{/if}
	{:else}
		<div class="flex flex-col gap-8">
			<Section label="Metadata">
				<div class="flex flex-col gap-6">
					<label class="flex flex-col gap-1">
						<span class="text-xs font-semibold text-emphasis">Summary</span>
						<!-- svelte-ignore a11y_autofocus -->
						<TextInput
							inputProps={{
								autofocus: true,
								type: 'text',
								placeholder: 'Short summary to be displayed when listed',
								disabled: !can_write,
								onkeyup: () => {
									if (!edit && summary?.length > 0 && !dirtyPath) {
										pathC?.setName(
											summary
												.toLowerCase()
												.replace(/[^a-z0-9_]/g, '_')
												.replace(/-+/g, '_')
												.replace(/^-|-$/g, '')
										)
									}
								}
							}}
							bind:value={summary}
						/>
					</label>

					<div class="flex flex-col gap-1">
						<label for="path" class="text-xs font-semibold text-emphasis">Path</label>
						{#if !edit && !trigger?.isPrimary}
							<Path
								bind:dirty={dirtyPath}
								bind:this={pathC}
								checkInitialPathExistence={!edit}
								bind:error={pathError}
								bind:path
								{initialPath}
								namePlaceholder="schedule"
								kind="schedule"
								disableEditing={!can_write}
							/>
						{:else}
							<div class="flex justify-start w-full">
								<Badge
									color="gray"
									class={twMerge(
										'center-center !bg-surface-secondary !text-secondary rounded-r-none border',
										ButtonType.UnifiedMinHeightClasses['md']
									)}
								>
									Schedule path (not editable)
								</Badge>
								<input
									type="text"
									readonly
									value={path}
									size={path?.length || 50}
									class={twMerge(
										'font-mono !text-2xs grow shrink overflow-x-auto !py-0 !border-l-0 !rounded-l-none',
										ButtonType.UnifiedMinHeightClasses['md']
									)}
									onfocus={({ currentTarget }) => {
										currentTarget.select()
									}}
								/>
								<!-- <span class="font-mono text-sm break-all">{path}</span> -->
							</div>
						{/if}
					</div>

					<label class="flex flex-col gap-1">
						<span class="text-xs font-semibold text-emphasis">Description</span>
						<textarea
							rows="4"
							use:autosize
							bind:value={description}
							placeholder="What this schedule does and how to use it"
							disabled={!can_write}
						></textarea>
					</label>
				</div>
			</Section>

			<Section label="Schedule">
				{#snippet header()}
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
				{/snippet}
				<div class="flex flex-col gap-6">
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
					<div class="flex flex-col gap-1">
						<Toggle
							options={{
								right: 'Pause schedule until...',
								rightTooltip:
									'Pausing the schedule will program the next job to run as if the schedule starts at the time the pause is lifted, instead of now.'
							}}
							bind:checked={showPauseUntil}
							disabled={!can_write}
						/>
						{#if showPauseUntil}
							<DateTimeInput bind:value={paused_until} />
						{/if}
					</div>
				</div>
			</Section>

			<Section label="Runnable">
				{#if !hideTarget}
					{#if !edit}
						<p class="text-xs mb-1 text-secondary">
							Pick a script or flow to be triggered by the schedule<Required required={true} />
						</p>
						<ScriptPicker
							disabled={(initialScriptPath != '' && !initNewPath) || !can_write}
							initialPath={initialScriptPath}
							kinds={['script']}
							allowFlow={true}
							allowRefresh={can_write}
							bind:itemKind
							bind:scriptPath={script_path}
							on:select={(e) => {
								loadScript(e.detail.path)
							}}
							clearable
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
				{/if}
				<div class={!hideTarget ? 'mt-6' : ''}>
					{#if !loading}
						{#if runnable || draftSchema}
							{@const schema = draftSchema ?? runnable?.schema}
							{#if schema && schema.properties && Object.keys(schema.properties).length > 0}
								{#await import('$lib/components/SchemaForm.svelte')}
									<Loader2 class="animate-spin" />
								{:then Module}
									<Module.default
										showReset
										onlyMaskPassword
										disabled={!can_write}
										schema={$state.snapshot(schema)}
										bind:isValid
										bind:args
									/>
								{/await}
							{:else}
								<div class="text-xs text-secondary">
									This {is_flow ? 'flow' : 'script'} takes no argument
								</div>
							{/if}
						{:else if script_path != ''}
							<div class="text-xs text-secondary my-2">
								You cannot see the the {is_flow ? 'flow' : 'script'} input form as you do not have access
								to it.
							</div>
						{:else}
							<div class="text-xs text-secondary my-2">
								Pick a {is_flow ? 'flow' : 'script'} and fill its argument here
							</div>
						{/if}
					{:else}
						<Loader2 class="animate-spin" />
					{/if}
				</div>
			</Section>

			<Section label="Advanced" collapsable>
				{@render errorHandler()}
			</Section>
		</div>
	{/if}
{/snippet}

{#snippet errorHandler()}
	<div class="flex flex-col gap-2 min-h-96">
		{#if !loading}
			<Tabs bind:selected={optionTabSelected}>
				<Tab value="error_handler" label="Error Handler" />
				<Tab value="recovery_handler" label="Recovery Handler" />
				<Tab value="success_handler" label="Success Handler" />
				<Tab value="retries" label="Retries" />
				<Tab value="dynamic_skip" label="Dynamic skip" />
				{#if itemKind === 'script'}
					<Tab value="tag" label="Custom tag" />
				{/if}
			</Tabs>
			{#if optionTabSelected === 'error_handler'}
				<Section label="Error handler">
					{#snippet header()}
						<div class="flex flex-row gap-2">
							{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
						</div>
					{/snippet}
					{#snippet action()}
						<div class="flex flex-row items-center gap-1 text-xs text-secondary">
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
								{#snippet buttonReplacement()}
									<Save size={12} class="mr-1" />
									Set as default
								{/snippet}
							</Dropdown>
						</div>
					{/snippet}
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
						toggleText="Alert channel on error"
						customScriptTemplate="/scripts/add?hub=hub%2F19743%2Fwindmill%2Fschedule_error_handler_template"
						bind:customHandlerKind={errorHandleritemKind}
						bind:handlerExtraArgs={errorHandlerExtraArgs}
					>
						{#snippet customTabTooltip()}
							<Tooltip>
								<div class="flex gap-20 items-start mt-3">
									<div class="text-xs"
										>The following args will be passed to the error handler:
										<ul class="mt-1 ml-2">
											<li
												><b>workspace_id</b>: The ID of the workspace that the schedule belongs to.</li
											>
											<li><b>job_id</b>: The UUID of the job that errored.</li>
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
						{/snippet}
					</ErrorOrRecoveryHandler>
					<div class="flex flex-row items-center justify-between">
						<div class="flex flex-row items-center mt-4 font-semibold text-xs gap-2">
							<p class={emptyString(errorHandlerPath) ? 'text-primary' : ''}>
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
								class="!w-14 text-center {emptyString(errorHandlerPath) ? 'text-primary' : ''}"
								bind:value={failedTimes}
								disabled={!$enterpriseLicense}
								min="1"
							/>
							<p class={emptyString(errorHandlerPath) ? 'text-primary' : ''}
								>time{failedTimes > 1 ? 's in a row' : ''}</p
							>
						</div>
					</div>
				</Section>
			{:else if optionTabSelected === 'recovery_handler'}
				{@const disabled = !can_write || emptyString($enterpriseLicense)}
				<Section label="Recovery handler">
					{#snippet header()}
						<div class="flex flex-row gap-2">
							{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
						</div>
					{/snippet}
					{#snippet action()}
						<div class="flex flex-row items-center text-secondary text-xs gap-2">
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
								{#snippet buttonReplacement()}
									<Save size={12} class="mr-1" />
									Set as default
								{/snippet}
							</Dropdown>
						</div>
					{/snippet}
					<ErrorOrRecoveryHandler
						isEditable={!disabled}
						errorOrRecovery="recovery"
						bind:handlerSelected={recoveryHandlerSelected}
						bind:handlerPath={recoveryHandlerPath}
						toggleText="Alert channel when error recovered"
						customScriptTemplate="/scripts/add?hub=hub%2F9082%2Fwindmill%2Fschedule_recovery_handler_template"
						bind:customHandlerKind={recoveryHandlerItemKind}
						bind:handlerExtraArgs={recoveryHandlerExtraArgs}
					>
						{#snippet customTabTooltip()}
							<Tooltip>
								<div class="flex gap-20 items-start mt-3">
									<div class="text-xs"
										>The following args will be passed to the recovery handler:
										<ul class="mt-1 ml-2">
											<li><b>path</b>: The path of the script or flow that recovered.</li>
											<li><b>is_flow</b>: Whether the runnable is a flow.</li>
											<li><b>schedule_path</b>: The path of the schedule.</li>
											<li><b>error</b>: The error of the last job that errored</li>
											<li
												><b>error_started_at</b>: The start datetime of the last job that errored</li
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
						{/snippet}
					</ErrorOrRecoveryHandler>
					<div class="flex flex-row items-center justify-between">
						<div
							class="flex flex-row items-center mt-5 font-semibold text-xs {emptyString(
								recoveryHandlerPath
							)
								? 'text-primary'
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
					{#snippet header()}
						<div class="flex flex-row gap-2">
							{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
						</div>
					{/snippet}
					{#snippet action()}
						<div class="flex flex-row items-center text-secondary text-xs gap-2">
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
								{#snippet buttonReplacement()}
									<Save size={12} class="mr-1" />
									Set as default
								{/snippet}
							</Dropdown>
						</div>
					{/snippet}
					<ErrorOrRecoveryHandler
						isEditable={!disabled}
						errorOrRecovery="success"
						bind:handlerSelected={successHandlerSelected}
						bind:handlerPath={successHandlerPath}
						toggleText="Alert channel when successful"
						customScriptTemplate="/scripts/add?hub=hub%2F9071%2Fwindmill%2Fschedule_success_handler_template"
						bind:customHandlerKind={successHandlerItemKind}
						bind:handlerExtraArgs={successHandlerExtraArgs}
					>
						{#snippet customTabTooltip()}
							<Tooltip>
								<div class="flex gap-20 items-start mt-3">
									<div class="text-xs"
										>The following args will be passed to the success handler:
										<ul class="mt-1 ml-2">
											<li><b>path</b>: The path of the script or flow that succeeded.</li>
											<li><b>is_flow</b>: Whether the runnable is a flow.</li>
											<li><b>schedule_path</b>: The path of the schedule.</li>
											<li><b>success_result</b>: The result of the successful job</li>
											<li><b>success_started_at</b>: The start datetime of the successful job</li>
										</ul>
									</div>
								</div>
							</Tooltip>
						{/snippet}
					</ErrorOrRecoveryHandler>
				</Section>
			{:else if optionTabSelected === 'retries'}
				{@const disabled = !can_write || emptyString($enterpriseLicense)}
				<Section label="Retries">
					{#snippet header()}
						<div class="flex flex-row gap-2">
							{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
						</div>
						<Tooltip>
							If defined, upon error this schedule will be retried with a delay and a maximum number
							of attempts as defined below.
							<br />
							This is only available for individual script. For flows, retries can be set on each flow
							step in the flow editor.
						</Tooltip>
					{/snippet}
					{#if itemKind !== 'script'}
						<Alert type="info" title="Only available for scripts" class="mb-2">
							Error Handler and Retries are only available for scripts. For flows, use the built-in <a
								href="https://www.windmill.dev/docs/flows/flow_error_handler"
								target="_blank">error handler</a
							>
							and <a href="https://www.windmill.dev/docs/flows/retries" target="_blank">retries</a>.
						</Alert>
					{:else}
						<FlowRetries
							bind:flowModuleRetry={retry}
							disabled={itemKind !== 'script' || disabled}
						/>
					{/if}
				</Section>
			{:else if optionTabSelected === 'dynamic_skip'}
				<Section label="Dynamic skip">
					{#snippet header()}
						<Tooltip>
							Optional script to filter scheduled dates. Receives the proposed datetime and returns
							boolean. True = run on this date, False = skip to next occurrence.
						</Tooltip>
					{/snippet}
					<div class="flex flex-col gap-6">
						<Label label="Dynamic skip script">
							<div class="flex flex-row">
								<ScriptPicker
									disabled={!can_write}
									bind:scriptPath={dynamicSkipPath}
									kinds={['script']}
									allowRefresh={can_write}
									clearable
								/>
								{#if !dynamicSkipPath}
									<Button
										btnClasses="ml-4 whitespace-nowrap"
										variant="default"
										size="xs"
										href="/scripts/add?hub=hub%2F19822%2Fwindmill%2Fdynamic_skip_template"
										disabled={!can_write}
										target="_blank"
									>
										Create from template
									</Button>
								{/if}
							</div>
						</Label>
						<Alert type="info" size="xs" title="Handler requirements">
							Handler must return a boolean value. Return true to execute the scheduled job, false
							to skip.
						</Alert>
					</div>
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
{/snippet}

{#if useDrawer}
	<Drawer size="900px" bind:this={drawer}>
		<DrawerContent
			title={edit
				? can_write
					? `Edit schedule ${initialPath}`
					: `View schedule ${initialPath}`
				: 'New schedule'}
			on:close={drawer.closeDrawer}
		>
			{#snippet actions()}
				<div class="flex flex-row gap-4 items-center">
					{@render saveButton()}
				</div>
			{/snippet}
			{@render content()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'Schedule' : ''} headerClass="grow min-w-0 h-[30px]">
		{#snippet header()}
			{#if customLabel}
				{@render customLabel()}
			{/if}
		{/snippet}
		{#snippet action()}
			<div class="flex flex-row gap-2 items-center">
				{@render saveButton()}
			</div>
		{/snippet}
		{#if docDescription}
			{@render docDescription()}
		{/if}
		{@render content()}
	</Section>
{/if}
