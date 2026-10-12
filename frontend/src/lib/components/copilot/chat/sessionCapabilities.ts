import type { Tool } from './shared'

// A leaf on purpose: every module defining a session-reachable tool imports these
// constants, so importing anything with side effects here drags it into all of them.

/**
 * What a user may do in ONE workspace, as an AI session's toolset needs to know it.
 * Best-effort, not a boundary — the token is the enforcement point, so this narrows
 * what the model is offered and guarantees nothing.
 */
export type SessionCapability =
	/** Passes drafts.rs `require_can_write_path` — for a draft of any kind, schedules and
	 * triggers included, though an operator may create those directly. */
	| 'write_draft'
	/** Passes `require_can_write_path` for a flow draft, which an operator also passes where the
	 * workspace grants `builder_flows`. The server then refuses a draft carrying inline code. */
	| 'write_flow_draft'
	/** Passes the operator refusal jobs.rs `run_preview_*` makes before starting a job. */
	| 'run_preview'
	/** Passes `run_preview_flow`'s `check_operator_can_build_flows`: composed flows only, for an
	 * operator with `builder_flows`. */
	| 'run_flow_preview'
	/** Passes `check_deploy_rules`. */
	| 'deploy'
	/** Passes the operator refusal the script and app handlers make before creating or
	 * deleting one. The same condition as `run_preview` today, but another handler family's
	 * check, so each follows its own gate if the two ever diverge. */
	| 'manage_code'
	/** Passes the flow handlers' `check_operator_can_build_flows`. */
	| 'manage_flows'
	/** Passes `gate_operator_writes` on the schedule router: withheld only from an operator whose
	 * workspace set `operator_settings.manage_schedules` to false. */
	| 'manage_schedules'
	/** The same gate on the trigger routers, read from `manage_triggers`. */
	| 'manage_triggers'
	/** Passes `require_admin`. */
	| 'admin'

export type SessionAccess = ReadonlySet<SessionCapability>

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
	return access.has(kind === 'flow' ? 'write_flow_draft' : 'write_draft')
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
}
