import type { Tool } from './shared'

// A leaf on purpose: every module defining a session-reachable tool imports these
// constants, so importing anything with side effects here drags it into all of them.

/**
 * What a user may do in ONE workspace, as an AI session's toolset needs to know it.
 * Best-effort, not a boundary — the token is the enforcement point, so this narrows
 * what the model is offered and guarantees nothing.
 */
export type SessionCapability =
	/** Passes drafts.rs `require_can_write_path` for a script draft: the role's `code`. A tool
	 * writing another kind requires that kind's capability below instead. */
	| 'write_draft'
	/** Passes `require_can_write_path` for a flow draft: `flows` at `compose` or above. At
	 * `compose` the server then refuses a draft carrying inline code. */
	| 'write_flow_draft'
	/** The same for a raw app draft, under `apps`. */
	| 'write_app_draft'
	/** Inline code in a flow draft: `flows` at `full`. At `compose` the server validates the draft
	 * as a pure composition, even for a role that may write scripts. */
	| 'write_inline_flow_code'
	/** Inline and hub runnables in an app draft: `apps` at `full`. */
	| 'write_inline_app_code'
	/** Passes `require_can_write_path` for a schedule draft: the role's `manage_schedules`. */
	| 'write_schedule_draft'
	/** The same for a trigger draft, read from `manage_triggers`. */
	| 'write_trigger_draft'
	/** Passes `require_can_write_path` for a resource draft, which every role does. */
	| 'write_resource_draft'
	/** The same for a variable draft. */
	| 'write_variable_draft'
	/** Passes the `code` refusal jobs.rs `run_preview_*` makes before starting a job. */
	| 'run_preview'
	/** Passes `run_preview_flow`'s `check_operator_can_build`: composed flows only at `compose`. */
	| 'run_flow_preview'
	/** Passes `check_deploy_rules`. */
	| 'deploy'
	/** Passes the `code` refusal the script handlers make before creating or
	 * deleting one. The same condition as `run_preview` today, but another handler family's
	 * check, so each follows its own gate if the two ever diverge. */
	| 'manage_code'
	/** Passes the flow handlers' `check_operator_can_build`. */
	| 'manage_flows'
	/** Passes the raw app handlers' `check_operator_can_build`, read from `apps`. */
	| 'manage_apps'
	/** Passes `gate_operator_writes` on the schedule router: the role's `manage_schedules`. */
	| 'manage_schedules'
	/** The same gate on the trigger routers, read from `manage_triggers`. */
	| 'manage_triggers'
	/** Passes `require_admin`. */
	| 'admin'
	/** Withheld when the workspace refuses this user's direct deploys through
	 * `DisableDirectDeployment` (a production workspace): the session is then read-only and
	 * ships only the tools `readOnlyToolAllowed` admits. Held in every other session. */
	| 'modify_workspace'

export type SessionAccess = ReadonlySet<SessionCapability>

/** Whether `access` is a read-only session. `undefined` access answers false, as the toolset
 * filter does. */
export function isReadOnlyAccess(access: SessionAccess | undefined): boolean {
	return !!access && !access.has('modify_workspace')
}

/** What a session in Use mode holds whatever the user's role: nothing, as in a protected
 * workspace. Fixed rather than narrowed from the resolved access, so Use ships no write while
 * that check is still in flight. */
export const USE_MODE_ACCESS: SessionAccess = new Set()

/** Whether `access` builds anything: a draft of code, a flow, an app, a schedule or a trigger.
 * Resource and variable drafts do not count, since every role writes them. `undefined` access
 * answers true, as the toolset filter does. */
export function canBuild(access: SessionAccess | undefined): boolean {
	return (
		!access ||
		(
			[
				'write_draft',
				'write_flow_draft',
				'write_app_draft',
				'write_schedule_draft',
				'write_trigger_draft'
			] as const
		).some((c) => access.has(c))
	)
}

/**
 * The capabilities without which a tool's call cannot succeed — usually because the
 * backend refuses it, occasionally because only that capability produces the tool's
 * input. Never a relevance judgement: a tool the server would accept ships, even when
 * it is of little use to the session, because withholding it is a stricter answer than
 * the one the user would get by trying.
 */
export type SessionToolPolicy = readonly SessionCapability[]

export const NONE: SessionToolPolicy = []
export const WRITE_DRAFT: SessionToolPolicy = ['write_draft']
export const WRITE_FLOW_DRAFT: SessionToolPolicy = ['write_flow_draft']
export const WRITE_APP_DRAFT: SessionToolPolicy = ['write_app_draft']
export const WRITE_SCHEDULE_DRAFT: SessionToolPolicy = ['write_schedule_draft']
export const WRITE_TRIGGER_DRAFT: SessionToolPolicy = ['write_trigger_draft']
export const WRITE_RESOURCE_DRAFT: SessionToolPolicy = ['write_resource_draft']
export const WRITE_VARIABLE_DRAFT: SessionToolPolicy = ['write_variable_draft']
export const RUN_PREVIEW: SessionToolPolicy = ['run_preview']
export const RUN_FLOW_PREVIEW: SessionToolPolicy = ['run_flow_preview']
export const DEPLOY: SessionToolPolicy = ['deploy']

/** Editors a session can open. Every editor autosaves a draft, so it needs the draft
 * capability of its kind; a pipeline's nodes are script drafts. */
export type EditorKind = 'script' | 'flow' | 'raw_app' | 'pipeline'

/** Whether `access` can save what the `kind` editor writes. `undefined` access (not
 * resolved yet, or not a session) answers true, as the toolset filter does. */
export function canOpenEditor(access: SessionAccess | undefined, kind: EditorKind): boolean {
	if (!access) return true
	return access.has(
		kind === 'flow' ? 'write_flow_draft' : kind === 'raw_app' ? 'write_app_draft' : 'write_draft'
	)
}

/**
 * A tool that can reach an AI session's toolset. `requires` is mandatory, and declared
 * beside the `fn` whose call is what proves it right.
 */
export type SessionTool<T> = Tool<T> & {
	requires: SessionToolPolicy
	/** For a tool taking the item kind as its `type` argument, where one verdict for the whole
	 * tool is either too strict or too loose: what each kind's handler needs. A session is
	 * offered only the kinds it can land, and the rest never appear in the schema it sees. */
	kindRequires?: Readonly<Partial<Record<string, SessionToolPolicy>>>
	/** Ships in a read-only session although it is not `planModeSafe`: a view tool that opens
	 * no editor there, or a run of a deployed item, which a production workspace is for. */
	readOnlySafe?: boolean
}
