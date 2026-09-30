<script lang="ts">
	import { Alert, Badge, Button, ButtonType } from '$lib/components/common'
	import {
		clearPageDrawerAnchor,
		setPageDrawerAnchor
	} from '$lib/components/sessions/pageDrawerSession'
	import { SCHEDULES_PATH } from '$lib/components/sessions/previewPaths'
	import TriggerAdvancedBadges from '../TriggerAdvancedBadges.svelte'
	import ScheduleAdvancedOptions, { scheduleAdvancedCfg } from './ScheduleAdvancedOptions.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import CronInput from '$lib/components/CronInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { loadSchema } from '$lib/infer'
	import PipelineLockedRunnableInfo from '$lib/components/triggers/PipelineLockedRunnableInfo.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
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
	import { canWrite, emptyString, formatCron, sendUserToast, cronV1toV2 } from '$lib/utils'
	import { useScheduleLock } from '$lib/operatorWriteRights'
	import { base } from '$lib/base'
	import Section from '$lib/components/Section.svelte'
	import { List, Loader2, AlertTriangle } from 'lucide-svelte'
	import autosize from '$lib/autosize'
	import TriggerEditorToolbar from '$lib/components/triggers/TriggerEditorToolbar.svelte'
	import { saveScheduleFromCfg } from '$lib/components/flows/scheduleUtils'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'
	import { runScheduleNow } from '../scheduled/utils'
	import { handleConfigChange } from '../utils'
	import { withForkConflictRetry } from '$lib/utils/forkConflict'
	import { useTriggerDraftSync } from '../useTriggerDraftSync.svelte'
	import LocalDraftBanner from '$lib/components/LocalDraftBanner.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { twMerge } from 'tailwind-merge'
	import PermissionedAsLine from '../PermissionedAsLine.svelte'
	import { useActingUser } from '$lib/actingUser.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	const scheduleLock = useScheduleLock()

	let {
		useDrawer = true,
		inline = false,
		onClose = undefined,
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
	// When non-empty, the drawer was opened from the pipeline editor for an
	// already-bound script. We swap the runnable ScriptPicker for a read-only
	// viewer so the trigger can't be silently reassigned off the pipeline.
	let fixedScriptPath = $state('')
	let runnable: Pick<Script | Flow, 'schema'> | undefined = $state()
	let args: Record<string, any> = $state({})
	let loading = $state(false)
	let drawerLoading = $state(true)
	let showLoading = $state(false)
	let initialConfig: Record<string, any> | undefined = undefined
	let extraPerms: Record<string, boolean> = $state({})
	// Path the permissions above were loaded for — the verdict is about the schedule as
	// stored, not about a rename being typed into the form. `undefined` until a config has
	// been loaded, when there is no deployed schedule to deny access to.
	let permsPath: string | undefined = $state(undefined)
	let initNewPath = $state(false)
	let path: string = $state('')
	let enabled: boolean = $state(false)
	let pathError = $state('')
	let summary = $state('')
	let labels: string[] | undefined = $state(undefined)
	let description = $state('')
	let no_flow_overlap = $state(false)
	let tag: string | undefined = $state(undefined)
	let validCRON = $state(true)
	let isValid = $state(true)
	let allowSchedule = $derived(isValid && validCRON && script_path != '')
	let deploymentLoading = $state(false)
	// Set by `openNew({ onSaveDraft })`: the caller keeps the schedule as its own
	// draft, so Save hands it the config instead of writing it, and nothing may
	// autosave a workspace trigger draft for a path that is not deployed.
	let saveDraftHandler: ((cfg: Record<string, any>) => void) | undefined = $state(undefined)
	let permissionedAs = $state<string | undefined>(undefined)
	let selectedPermissionedAs = $state<string | undefined>(undefined)
	let preservePermissionedAs = $state(false)

	const operatingWorkspace = useOperatingWorkspace()
	const wsId = $derived($operatingWorkspace)
	// `undefined` while the lookup is in flight or after it failed; the checks below then
	// refuse rather than fall back to rights that belong to another workspace.
	const acting = useActingUser(() => wsId)
	const actingUser = $derived(acting.current)
	const can_write = $derived(
		(permsPath === undefined || canWrite(permsPath, extraPerms, actingUser)) && !$scheduleLock
	)
	// Editing the runnable is closed to operators, and an unresolved acting user is no
	// evidence that this one isn't.
	const canEditRunnable = $derived(actingUser !== undefined && !actingUser.operator)
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

	const draftSync = useTriggerDraftSync({
		itemKind: 'trigger_schedule',
		path: () => (saveDraftHandler ? '' : initialPath),
		workspace: () => wsId,
		drawerLoading: () => drawerLoading,
		getCfg: () => scheduleCfg,
		applyCfg: loadScheduleCfg,
		deployed: () => initialConfig
	})

	export async function openEdit(
		ePath: string,
		isFlow: boolean,
		defaultCfg?: Record<string, any>,
		fixedScriptPath_?: string
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		acting.forgetFailures()
		saveDraftHandler = undefined
		try {
			drawer?.openDrawer()
			setPageDrawerAnchor(SCHEDULES_PATH, ePath)
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			path = defaultCfg?.path ?? ePath
			fixedScriptPath = fixedScriptPath_ ?? ''
			const { overlay: draftOverlay, noDeployed } = await loadSchedule(defaultCfg)
			// Draft-only schedules have no deployed row, so saving must CREATE (update 404s).
			edit = !noDeployed
			if (!defaultCfg) {
				// Form holds DEPLOYED here; capture it as `initialConfig` so the
				// dirty check / banner fires whenever a saved draft exists.
				initialConfig = structuredClone($state.snapshot(getScheduleCfg()))
			}
			if (draftOverlay) await loadScheduleCfg(draftOverlay)
			await draftSync.maybeRestore()
		} finally {
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
			if (wsId) {
				defaultErrorHandlerMaybe = (await SettingService.getGlobal({
					key: 'default_error_handler_' + wsId!
				})) as any
				defaultRecoveryHandlerMaybe = (await SettingService.getGlobal({
					key: 'default_recovery_handler_' + wsId!
				})) as any
				defaultSuccessHandlerMaybe = (await SettingService.getGlobal({
					key: 'default_success_handler_' + wsId!
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
		schedule_path?: string,
		fixedScriptPath_?: string,
		opts: { getDraft?: boolean; onSaveDraft?: (cfg: Record<string, any>) => void } = {}
	) {
		const getDraft = opts.getDraft ?? true
		saveDraftHandler = opts.onSaveDraft
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		acting.forgetFailures()
		try {
			let s: Schedule | undefined
			if (schedule_path) {
				const resp = await ScheduleService.getSchedule({
					workspace: wsId!,
					path: schedule_path,
					getDraft
				})
				// `.draft` holds the saved Schedule; layer it over the deployed
				// fields so the form assignments below see the last-saved state.
				const { draft: draftFromBackend, ...deployedSchedule } = resp as any
				s = draftFromBackend
					? ({ ...deployedSchedule, ...draftFromBackend } as Schedule)
					: (deployedSchedule as Schedule)
				initNewPath = true
			} else if (defaultValues) {
				s = defaultValues
			}
			drawer?.openDrawer()
			runnable = undefined
			edit = false
			// No deployed baseline for a brand-new schedule. The editor instance
			// is reused across open() calls, so clear any baseline left by a prior
			// openEdit — otherwise the "unsaved changes" banner / dirty check would
			// compare against a stale config.
			initialConfig = undefined
			itemKind = (s?.is_flow ?? nis_flow) ? 'flow' : 'script'
			initialScriptPath = initial_script_path ?? ''
			fixedScriptPath = fixedScriptPath_ ?? ''
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
			labels = s?.labels ?? undefined
			description = s?.description ?? ''
			script_path = s?.script_path ?? initialScriptPath
			args = s?.args ?? {}
			tag = s?.tag ?? undefined

			await loadScript(script_path)

			no_flow_overlap = s?.no_flow_overlap ?? false
			wsErrorHandlerMuted = s?.ws_error_handler_muted ?? false
			retry = s?.retry ?? undefined
			dynamicSkipPath = s?.dynamic_skip ?? undefined

			await setScheduleHandler(s)
			permissionedAs = undefined
			selectedPermissionedAs = undefined
			preservePermissionedAs = false
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
					runnable = await FlowService.getFlowByPath({ workspace: wsId!, path: p })
				} else if (p.startsWith('hub/')) {
					runnable = await loadSchema(wsId!, p, 'hubscript')
				} else {
					runnable = await ScriptService.getScriptByPath({ workspace: wsId!, path: p })
				}
			} catch (err) {}
		} else {
			runnable = undefined
		}
	}

	/**
	 * Apply the deployed config to the form, then return the saved-draft overlay
	 * so the caller captures `initialConfig` from the deployed-only form BEFORE
	 * applying the draft, making the banner fire whenever a draft is present.
	 */
	async function loadSchedule(
		defaultCfg?: Record<string, any>
	): Promise<{ overlay: Record<string, any> | undefined; noDeployed: boolean }> {
		if (defaultCfg) {
			await loadScheduleCfg(defaultCfg)
			return { overlay: undefined, noDeployed: false }
		}
		try {
			const s = await ScheduleService.getSchedule({
				workspace: wsId!,
				path: initialPath,
				getDraft: true
			})
			const { draft: draftFromBackend, ...deployedSchedule } = s as any
			await loadScheduleCfg(deployedSchedule)
			return {
				overlay: draftFromBackend
					? ({ ...deployedSchedule, ...draftFromBackend } as Record<string, any>)
					: undefined,
				// Draft-only: synthesized stand-in, no row, so saving must CREATE.
				noDeployed: !!(s as any).no_deployed
			}
		} catch (err) {
			sendUserToast(`Could not load schedule: ${err}`, true)
			return { overlay: undefined, noDeployed: false }
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
		labels = cfg.labels ?? undefined
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
		permsPath = cfg.path
		tag = cfg.tag
		permissionedAs = cfg.permissioned_as
		selectedPermissionedAs = cfg.permissioned_as
		preservePermissionedAs = !!cfg.permissioned_as

		loading = false
	}

	async function scheduleScript(): Promise<void> {
		const previousPath = initialPath
		const scheduleCfg = getScheduleCfg()
		if (saveDraftHandler) {
			saveDraftHandler($state.snapshot(scheduleCfg))
			drawer?.closeDrawer()
			return
		}
		deploymentLoading = true
		const isSaved = await saveScheduleFromCfg(scheduleCfg, edit, wsId!)
		if (isSaved) {
			draftSync.discard(previousPath, scheduleCfg)
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
				slack: '/workspace-or-schedule-error-handler-slack',
				email: '/workspace-or-error-handler-email'
			},
			recovery: {
				teams: '/schedule-recovery-handler-teams',
				slack: '/schedule-recovery-handler-slack',
				email: '/workspace-or-error-handler-email'
			},
			success: {
				teams: '/schedule-success-handler-teams',
				slack: '/schedule-success-handler-slack',
				email: '/workspace-or-error-handler-email'
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
			...scheduleAdvancedCfg({
				errorHandlerSelected,
				errorHandlerPath,
				errorHandleritemKind,
				errorHandlerExtraArgs,
				wsErrorHandlerMuted,
				failedTimes,
				failedExact,
				recoveryHandlerSelected,
				recoveryHandlerPath,
				recoveryHandlerItemKind,
				recoveryHandlerExtraArgs,
				recoveredTimes,
				successHandlerSelected,
				successHandlerPath,
				successHandlerItemKind,
				successHandlerExtraArgs,
				retry,
				dynamicSkipPath,
				tag
			}),
			summary: summary != '' ? summary : undefined,
			labels: labels,
			description: description,
			no_flow_overlap: no_flow_overlap,
			paused_until: paused_until,
			cron_version: cronVersion,
			extra_perms: extraPerms,
			permissioned_as: selectedPermissionedAs,
			preserve_permissioned_as: preservePermissionedAs || undefined
		}
	}

	async function handleToggleEnabled(nEnabled: boolean) {
		const previousEnabled = enabled
		const writesBackend = !trigger?.draftConfig
		const togglePath = initialPath
		const setEnabled = (v: boolean) => {
			// The drawer is reused: a revert landing after it moved to another
			// schedule would fold this one's value into that one's baseline.
			if (initialPath !== togglePath) return
			enabled = v
			if (writesBackend) draftSync.patchBaseline({ enabled: v })
		}
		setEnabled(nEnabled)
		if (writesBackend) {
			let ok: boolean
			try {
				ok = await withForkConflictRetry(
					(force) =>
						ScheduleService.setScheduleEnabled({
							path: initialPath,
							workspace: wsId ?? '',
							requestBody: { enabled: nEnabled, force }
						}),
					'schedule'
				)
			} catch (err) {
				setEnabled(previousEnabled)
				throw err
			}
			if (!ok) {
				setEnabled(previousEnabled)
				return
			}
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
			triggerPath={initialPath}
			triggerKind="schedule"
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
			saveLabel={saveDraftHandler ? 'Save draft' : undefined}
		>
			{#snippet extra()}
				{#if !drawerLoading && edit}
					<div class="mr-12 flex flex-row gap-3">
						<Button
							size="sm"
							variant="default"
							startIcon={{ icon: List }}
							disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
							href={`${base}/runs/?schedule_path=${path}&job_trigger_kind=schedule&show_future_jobs=true`}
						>
							View runs
						</Button>
						<Button
							size="sm"
							variant="default"
							disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
							on:click={() => {
								runScheduleNow(script_path, path, is_flow, wsId!)
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
		<PermissionedAsLine
			{permissionedAs}
			{path}
			onPermissionedAsChange={(pa, preserve) => {
				selectedPermissionedAs = pa
				preservePermissionedAs = preserve
			}}
		/>
		<div class="flex flex-col gap-8">
			<Section headless>
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
					<LabelsInput bind:labels class="-mt-4" />

					<div class="flex flex-col gap-1">
						<label for="path" class="text-xs font-semibold text-emphasis">Path</label>
						{#if !edit && !trigger?.isPrimary}
							<Path
								workspaceOverride={wsId}
								bind:dirty={dirtyPath}
								bind:this={pathC}
								checkInitialPathExistence={!edit}
								bind:error={pathError}
								bind:path
								{initialPath}
								namePlaceholder="schedule"
								kind="schedule"
								disableEditing={!can_write}
								actingUser={actingUser ?? null}
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
					{#if fixedScriptPath != ''}
						<PipelineLockedRunnableInfo path={fixedScriptPath} />
					{:else if !edit}
						<p class="text-xs mb-1 text-secondary">
							Pick a script or flow to be triggered by the schedule<Required required={true} />
						</p>
						<ScriptPicker
							workspace={wsId}
							disabled={(initialScriptPath != '' && !initNewPath) || !can_write}
							initialPath={initialScriptPath}
							kinds={['script']}
							allowFlow={true}
							allowHub={true}
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
							workspace={wsId}
							disabled
							initialPath={script_path}
							scriptPath={script_path}
							allowFlow={true}
							{itemKind}
							allowView={script_path != '' && !!runnable}
							allowEdit={script_path != '' && !!runnable && canEditRunnable}
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
				{#snippet header()}
					<TriggerAdvancedBadges
						error_handler_path={errorHandlerPath}
						{retry}
						extraBadges={[
							{ name: 'Recovery Handler', active: !!recoveryHandlerPath },
							{ name: 'Success Handler', active: !!successHandlerPath },
							{ name: 'Dynamic Skip', active: !!dynamicSkipPath },
							{ name: 'Custom Tag', active: !!tag }
						]}
					/>
				{/snippet}
				<ScheduleAdvancedOptions
					{wsId}
					{itemKind}
					canWrite={can_write}
					{loading}
					bind:errorHandlerSelected
					bind:errorHandlerPath
					bind:errorHandleritemKind
					bind:errorHandlerExtraArgs
					bind:wsErrorHandlerMuted
					bind:failedTimes
					bind:failedExact
					bind:recoveryHandlerSelected
					bind:recoveryHandlerPath
					bind:recoveryHandlerItemKind
					bind:recoveryHandlerExtraArgs
					bind:recoveredTimes
					bind:successHandlerSelected
					bind:successHandlerPath
					bind:successHandlerItemKind
					bind:successHandlerExtraArgs
					bind:retry
					bind:dynamicSkipPath
					bind:tag
				/>
			</Section>
			<div class="pb-8" />
		</div>
	{/if}
{/snippet}

{#snippet drawerBody()}
	<DrawerContent
		hideClose={inline && !onClose}
		fullScreen={!inline}
		bannerReserved={draftSync.hasBaseline}
		title={edit
			? can_write
				? `Edit schedule ${initialPath}`
				: `View schedule ${initialPath}`
			: saveDraftHandler
				? 'Draft schedule'
				: 'New schedule'}
		on:close={() => (inline ? onClose?.() : drawer?.closeDrawer())}
	>
		{#snippet actions()}
			<div class="flex flex-row gap-4 items-center">
				{@render saveButton()}
			</div>
		{/snippet}
		{#snippet banner()}
			<LocalDraftBanner
				show={draftSync.hasDraft}
				getDeployed={() => draftSync.deployed}
				reserveSpace={draftSync.hasBaseline}
				getCurrent={() => draftSync.current}
				onDiscard={() => draftSync.resetToDeployed(initialPath)}
				disabled={!can_write}
			/>
		{/snippet}
		{@render content()}
	</DrawerContent>
{/snippet}

{#if useDrawer && inline}
	{@render drawerBody()}
{:else if useDrawer}
	<Drawer size="900px" bind:this={drawer} on:close={() => clearPageDrawerAnchor(SCHEDULES_PATH)}>
		{@render drawerBody()}
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
