import { hasCapabilities, type SessionAccess, type SessionCapability } from './sessionAccess'

/**
 * Why a tool may be withheld from an AI session. The two axes are deliberately
 * separate: `requires` is auditable authorization metadata (the backend would
 * refuse the call), `relevance` is context economy (the call would succeed but
 * the tool exists only to serve authoring). Collapsing them into one field makes
 * the permission vocabulary lie — `search_npm_packages` is permitted for every
 * user, it is just dead weight in a session that cannot write a draft.
 */
export type SessionToolPolicy = {
	requires: readonly SessionCapability[]
	/** Dropped when the session cannot write drafts, regardless of `requires`. */
	relevance?: 'authoring'
}

const NONE: SessionToolPolicy = { requires: [] }
const AUTHORING_AID: SessionToolPolicy = { requires: [], relevance: 'authoring' }
const WRITE_DRAFT: SessionToolPolicy = { requires: ['write_draft'] }
const DEPLOY: SessionToolPolicy = { requires: ['deploy'] }
const RUN_PREVIEW: SessionToolPolicy = { requires: ['run_preview'] }

/**
 * Policy for every tool that can reach an AI session's toolset — the STATIC global
 * set plus the sources appended after it (pipeline, MCP, plan mode). Keyed by tool
 * name rather than declared on each tool object because those later sources are
 * built by factories in other modules: a field on `globalTools` alone would look
 * exhaustive while silently missing them.
 *
 * A tool with no entry here is withheld, so add the entry with the tool.
 */
export const SESSION_TOOL_POLICIES: Record<string, SessionToolPolicy> = {
	// ── Reads, docs and conversation ────────────────────────────────────────
	read_skill: NONE,
	open_page: NONE,
	askUserQuestion: NONE,
	update_user_instructions: NONE,
	search_docs: NONE,
	read_docs_page: NONE,
	list_workspace_items: NONE,
	read_workspace_item: NONE,
	read_flow_module_code: NONE,
	read_app_file: NONE,
	search_app: NONE,
	diff: NONE,
	read_file: NONE,
	search_files: NONE,

	// ── Run history ─────────────────────────────────────────────────────────
	list_runs: NONE,
	get_flow_run_details: NONE,
	get_job_logs: NONE,
	cancel_job: NONE,

	// ── Data ────────────────────────────────────────────────────────────────
	list_datatables: NONE,
	get_datatable_table_schema: NONE,
	list_ducklakes: NONE,
	exec_datatable_sql: RUN_PREVIEW,

	// ── API catalog and MCP ─────────────────────────────────────────────────
	// No capability needed: COVERED_ENDPOINTS in apiCatalogTools refuses the authoring
	// and delete endpoints for everyone, leaving reads and run-by-path. That list is
	// keyed by operationId and the server serves the catalog unfiltered, so it holds
	// only as long as it tracks the catalog — the server, not this table, is what
	// actually refuses a call that slips through.
	search_api_endpoints: NONE,
	call_api_get: NONE,
	call_api_endpoint: NONE,
	search_mcp_tools: NONE,
	call_mcp_read_tool: NONE,
	call_mcp_write_tool: NONE,

	// ── Session preview panel and artifacts ─────────────────────────────────
	open_preview: NONE,
	get_preview_status: NONE,
	close_page: NONE,
	get_app_runtime_logs: NONE,
	list_app_runs: NONE,
	search_dom: NONE,
	read_dom: NONE,
	take_screenshot: NONE,
	create_artifact: NONE,
	update_artifact: NONE,
	list_artifacts: NONE,
	read_artifact: NONE,
	list_artifact_versions: NONE,

	// ── Authoring aids: permitted for everyone, useless without write_draft ──
	get_instructions: AUTHORING_AID,
	search_hub_scripts: AUTHORING_AID,
	search_npm_packages: AUTHORING_AID,
	search_resource_types: AUTHORING_AID,
	get_trigger_schema: AUTHORING_AID,
	get_schedule_schema: AUTHORING_AID,
	get_db_schema: AUTHORING_AID,
	// Folder creation runs through the backend's `check_deploy_rules` (folders.rs
	// `create_folder`), and `folder` is one of the gated kinds — so a direct-deployment
	// lock refuses it even though a folder is not a deployed item. It is also useless
	// without something to put in it, hence the authoring relevance.
	create_folder: { requires: ['deploy_gated_kinds'], relevance: 'authoring' },
	// Ungated on purpose, for two reasons. Plan mode's deliverable is a plan artifact,
	// which is worth producing for someone else to execute even when this user can
	// change nothing themselves. And it is a posture the USER selects, so withholding
	// `exit_plan_mode` strands the model in it: the posture's instructions order it to
	// call that tool to hand the plan over, and nothing else ends the round.
	enter_plan_mode: NONE,
	exit_plan_mode: NONE,

	// ── Draft writes ────────────────────────────────────────────────────────
	// Every one of these funnels through the per-user draft lifecycle, which is
	// the single place the backend refuses (drafts.rs `require_can_write_path`) —
	// including the resource/variable/schedule/trigger tools, whose deployed-object
	// endpoints an operator's token would otherwise allow.
	write_script: WRITE_DRAFT,
	write_flow: WRITE_DRAFT,
	edit_script: WRITE_DRAFT,
	patch_flow_json: WRITE_DRAFT,
	set_flow_module_code: WRITE_DRAFT,
	write_schedule: WRITE_DRAFT,
	write_trigger: WRITE_DRAFT,
	write_resource: WRITE_DRAFT,
	write_variable: WRITE_DRAFT,
	init_app: WRITE_DRAFT,
	write_app_file: WRITE_DRAFT,
	patch_app_file: WRITE_DRAFT,
	delete_app_file: WRITE_DRAFT,
	write_app_runnable: WRITE_DRAFT,
	delete_app_runnable: WRITE_DRAFT,
	// Ungated on purpose: discarding your OWN draft skips `require_can_write_path`
	// (drafts.rs), and the exemption exists precisely so a user who has LOST write
	// access can still clean up drafts they left behind. Gating it here would strand
	// that cleanup. Rebasing is not exempt — it writes a fresh draft.
	discard_local_draft: NONE,
	rebase_draft: WRITE_DRAFT,

	// ── Deployed-object mutations ───────────────────────────────────────────
	// Both take the kind as an argument, so they stay available under a direct-deployment
	// lock: schedules and triggers are still deployable, and the prompt says which.
	deploy_workspace_item: DEPLOY,
	delete_workspace_item: DEPLOY,

	// ── Preview execution ───────────────────────────────────────────────────
	test_run_script: RUN_PREVIEW,
	test_run_flow: RUN_PREVIEW,
	test_run_step: RUN_PREVIEW,

	// ── Pipeline editor ─────────────────────────────────────────────────────
	get_pipeline_graph: NONE,
	read_pipeline_node: NONE,
	build_pipeline_node: WRITE_DRAFT,
	edit_pipeline_node: WRITE_DRAFT,
	remove_pipeline_node: WRITE_DRAFT,
	test_pipeline_node: RUN_PREVIEW
}

export function sessionToolAllowed(name: string, access: SessionAccess): boolean {
	const policy = SESSION_TOOL_POLICIES[name]
	// Fails closed, so a tool that ships without a policy disappears from restricted
	// sessions rather than leaking into them.
	if (!policy) return false
	if (policy.relevance === 'authoring' && !access.capabilities.has('write_draft')) {
		return false
	}
	return hasCapabilities(access, policy.requires)
}

/** Filter an assembled toolset. `access` undefined means "not resolved yet, or not
 * a session" — the toolset passes through untouched. */
export function filterSessionTools<T extends { def: { function: { name: string } } }>(
	tools: T[],
	access: SessionAccess | undefined
): T[] {
	if (!access) return tools
	return tools.filter((t) => sessionToolAllowed(t.def.function.name, access))
}
