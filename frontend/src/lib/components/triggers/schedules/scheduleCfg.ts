import { ScheduleService, SettingService, type ErrorHandler, type Retry } from '$lib/gen'
import { formatCron } from '$lib/utils'

export type ScheduleCfg = Record<string, any>

/** Every field of the schedule editor's form that a config reaches. */
export type ScheduleForm = {
	path: string
	cronVersion: string
	schedule: string
	timezone: string
	paused_until: string | undefined
	enabled: boolean
	summary: string
	labels: string[] | undefined
	description: string
	script_path: string
	itemKind: 'flow' | 'script'
	no_flow_overlap: boolean
	wsErrorHandlerMuted: boolean
	retry: Retry | undefined
	errorHandleritemKind: 'flow' | 'script'
	errorHandlerPath: string | undefined
	failedTimes: number
	failedExact: boolean
	errorHandlerExtraArgs: Record<string, any>
	errorHandlerSelected: ErrorHandler
	recoveryHandlerItemKind: 'flow' | 'script'
	recoveryHandlerPath: string | undefined
	recoveredTimes: number
	recoveryHandlerExtraArgs: Record<string, any>
	recoveryHandlerSelected: ErrorHandler
	successHandlerItemKind: 'flow' | 'script'
	successHandlerPath: string | undefined
	successHandlerExtraArgs: Record<string, any>
	successHandlerSelected: ErrorHandler
	dynamicSkipPath: string | undefined
	args: Record<string, any>
	extraPerms: Record<string, boolean>
	tag: string | undefined
	selectedPermissionedAs: string | undefined
	preservePermissionedAs: boolean
}

export function getHandlerType(
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

function splitHandler(handler: string | undefined): {
	kind: 'flow' | 'script'
	path: string | undefined
} {
	if (!handler) return { kind: 'script', path: undefined }
	const splitted = handler.split('/')
	return { kind: splitted[0] as 'flow' | 'script', path: splitted.slice(1)?.join('/') }
}

/** The form a config loads into. `path` is taken from the config: no rename reaches it. */
export function scheduleFormOf(cfg: ScheduleCfg): ScheduleForm {
	const onFailure = splitHandler(cfg.on_failure)
	const onRecovery = splitHandler(cfg.on_recovery)
	const onSuccess = splitHandler(cfg.on_success)
	return {
		path: cfg.path,
		cronVersion: cfg.cron_version ?? 'v2',
		schedule: cfg.schedule,
		timezone: cfg.timezone,
		paused_until: cfg.paused_until,
		enabled: cfg.enabled,
		summary: cfg.summary ?? '',
		labels: cfg.labels ?? undefined,
		description: cfg.description ?? '',
		script_path: cfg.script_path ?? '',
		itemKind: cfg.is_flow ? 'flow' : 'script',
		no_flow_overlap: cfg.no_flow_overlap ?? false,
		wsErrorHandlerMuted: cfg.ws_error_handler_muted ?? false,
		retry: cfg.retry,
		errorHandleritemKind: onFailure.kind,
		errorHandlerPath: onFailure.path,
		failedTimes: cfg.on_failure ? (cfg.on_failure_times ?? 1) : 1,
		failedExact: cfg.on_failure ? (cfg.on_failure_exact ?? false) : false,
		errorHandlerExtraArgs: cfg.on_failure ? (cfg.on_failure_extra_args ?? {}) : {},
		errorHandlerSelected: cfg.on_failure ? getHandlerType('error', onFailure.path ?? '') : 'slack',
		recoveryHandlerItemKind: onRecovery.kind,
		recoveryHandlerPath: onRecovery.path,
		recoveredTimes: cfg.on_recovery ? (cfg.on_recovery_times ?? 1) : 1,
		recoveryHandlerExtraArgs: cfg.on_recovery ? (cfg.on_recovery_extra_args ?? {}) : {},
		recoveryHandlerSelected: cfg.on_recovery
			? getHandlerType('recovery', onRecovery.path ?? '')
			: 'slack',
		successHandlerItemKind: onSuccess.kind,
		successHandlerPath: onSuccess.path,
		successHandlerExtraArgs: cfg.on_success ? (cfg.on_success_extra_args ?? {}) : {},
		successHandlerSelected: cfg.on_success
			? getHandlerType('success', onSuccess.path ?? '')
			: 'slack',
		dynamicSkipPath: cfg.dynamic_skip,
		args: cfg.args ?? {},
		extraPerms: cfg.extra_perms ?? {},
		tag: cfg.tag,
		selectedPermissionedAs: cfg.permissioned_as,
		preservePermissionedAs: !!cfg.permissioned_as
	}
}

/** The config the form describes: what a save sends and what the draft row holds. */
export function scheduleCfgOf(f: ScheduleForm): ScheduleCfg {
	return {
		path: f.path,
		schedule: formatCron(f.schedule),
		timezone: f.timezone,
		script_path: f.script_path,
		is_flow: f.itemKind === 'flow',
		args: f.args,
		enabled: f.enabled,
		on_failure: f.errorHandlerPath ? `${f.errorHandleritemKind}/${f.errorHandlerPath}` : undefined,
		on_failure_times: f.failedTimes,
		on_failure_exact: f.failedExact,
		on_failure_extra_args: f.errorHandlerPath ? f.errorHandlerExtraArgs : undefined,
		on_recovery: f.recoveryHandlerPath
			? `${f.recoveryHandlerItemKind}/${f.recoveryHandlerPath}`
			: undefined,
		on_recovery_times: f.recoveredTimes,
		on_recovery_extra_args: f.recoveryHandlerPath ? f.recoveryHandlerExtraArgs : {},
		on_success: f.successHandlerPath
			? `${f.successHandlerItemKind}/${f.successHandlerPath}`
			: undefined,
		on_success_extra_args: f.successHandlerPath ? f.successHandlerExtraArgs : {},
		ws_error_handler_muted: f.wsErrorHandlerMuted,
		retry: f.retry,
		summary: f.summary != '' ? f.summary : undefined,
		labels: f.labels,
		description: f.description,
		no_flow_overlap: f.no_flow_overlap,
		tag: f.tag,
		paused_until: f.paused_until,
		cron_version: f.cronVersion,
		extra_perms: f.extraPerms,
		dynamic_skip: f.dynamicSkipPath,
		permissioned_as: f.selectedPermissionedAs,
		preserve_permissioned_as: f.preservePermissionedAs || undefined
	}
}

/**
 * A schedule (or a draft layered over one) as the form would hand it back: the shape the
 * deployed side is kept in, so comparing it with what the form produces sees only real edits.
 */
export function normalizeScheduleCfg(raw: ScheduleCfg): ScheduleCfg {
	return scheduleCfgOf(scheduleFormOf(raw))
}

/**
 * A schedule that exists only as a draft, as deploying it will leave it: deploying one creates
 * it, and a create always enables it (`writeScheduleCfg`), so the draft has to carry the state
 * that create produces. Left as stored, a disabled draft would record `enabled: false` against
 * a server that says otherwise, and later updates omit the field, so nothing corrects it.
 */
export function draftOnlyScheduleCfg(cfg: ScheduleCfg): ScheduleCfg {
	return { ...cfg, enabled: true }
}

/** The handlers a new schedule starts with, from the workspace's defaults. */
async function workspaceDefaultHandlers(workspace: string): Promise<ScheduleCfg> {
	const [error, recovery, success] = (await Promise.all(
		['default_error_handler_', 'default_recovery_handler_', 'default_success_handler_'].map((k) =>
			SettingService.getGlobal({ key: k + workspace })
		)
	)) as any[]
	const cfg: ScheduleCfg = {}
	if (error != null) {
		cfg.ws_error_handler_muted = error['wsErrorHandlerMuted']
		cfg.on_failure = error['errorHandlerPath']
		cfg.on_failure_extra_args = error['errorHandlerExtraArgs']
		cfg.on_failure_times = error['failedTimes']
		cfg.on_failure_exact = error['failedExact']
	}
	if (recovery != null) {
		cfg.on_recovery = recovery['recoveryHandlerPath']
		cfg.on_recovery_extra_args = recovery['recoveryHandlerExtraArgs']
		cfg.on_recovery_times = recovery['recoveredTimes']
	}
	if (success != null) {
		cfg.on_success = success['successHandlerPath']
		cfg.on_success_extra_args = success['successHandlerExtraArgs']
	}
	return cfg
}

export type NewScheduleOptions = {
	workspace: string
	isFlow: boolean
	initialScriptPath: string
	/** Start from these values. */
	defaultValues?: ScheduleCfg
	/** Start from the schedule at this path (and the user's draft of it), under a new path. */
	schedulePath?: string
	getDraft: boolean
	/** The trigger is the runnable's primary schedule, which takes the runnable's path. */
	isPrimary: boolean
}

/** What a new schedule starts from. */
export async function newScheduleCfg(opts: NewScheduleOptions): Promise<ScheduleCfg> {
	let s: ScheduleCfg | undefined
	if (opts.schedulePath) {
		const resp = await ScheduleService.getSchedule({
			workspace: opts.workspace,
			path: opts.schedulePath,
			getDraft: opts.getDraft
		})
		// `.draft` holds the saved Schedule; layer it over the deployed fields.
		const { draft, ...deployed } = resp as any
		s = draft ? { ...deployed, ...draft } : deployed
	} else if (opts.defaultValues) {
		s = opts.defaultValues
	}
	const path = opts.schedulePath
		? ''
		: (opts.defaultValues?.path ?? (opts.isPrimary ? opts.initialScriptPath : ''))
	const handlers = s
		? {
				on_failure: s.on_failure,
				on_failure_times: s.on_failure_times,
				on_failure_exact: s.on_failure_exact,
				on_failure_extra_args: s.on_failure_extra_args,
				on_recovery: s.on_recovery,
				on_recovery_times: s.on_recovery_times,
				on_recovery_extra_args: s.on_recovery_extra_args,
				on_success: s.on_success,
				on_success_extra_args: s.on_success_extra_args,
				ws_error_handler_muted: s.ws_error_handler_muted ?? false
			}
		: await workspaceDefaultHandlers(opts.workspace)
	return normalizeScheduleCfg({
		ws_error_handler_muted: false,
		...handlers,
		path,
		schedule: s?.schedule ?? '0 0 12 * *',
		cron_version: s?.cron_version ?? 'v2',
		timezone: s?.timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone,
		paused_until: s?.paused_until ?? undefined,
		// A create always enables the schedule (`writeScheduleCfg`), so that is where a new one
		// starts: the deployed side a create records is the value it sent.
		enabled: true,
		summary: s?.summary ?? '',
		labels: s?.labels ?? undefined,
		description: s?.description ?? '',
		script_path: s?.script_path ?? opts.initialScriptPath,
		is_flow: s?.is_flow ?? opts.isFlow,
		args: s?.args ?? {},
		tag: s?.tag ?? undefined,
		no_flow_overlap: s?.no_flow_overlap ?? false,
		retry: s?.retry ?? undefined,
		dynamic_skip: s?.dynamic_skip
	})
}
