import type { SessionAccess, SessionCapability } from './sessionAccess'

/**
 * The capabilities without which a tool's call cannot succeed — usually because the
 * backend refuses it, occasionally because only that capability produces the tool's
 * input. Never a relevance judgement: a tool the server would accept ships, even when
 * it is of little use to the session, because withholding it is a stricter answer than
 * the one the user would get by trying.
 */
export type SessionToolPolicy = readonly SessionCapability[]

const NONE: SessionToolPolicy = []
const WRITE_DRAFT: SessionToolPolicy = ['write_draft']
const RUN_PREVIEW: SessionToolPolicy = ['run_preview']

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
	get_run: NONE,
	cancel_job: NONE,
	list_workers: NONE,

	// ── Data ────────────────────────────────────────────────────────────────
	list_datatables: NONE,
	get_datatable_table_schema: NONE,
	list_ducklakes: NONE,
	list_data_metrics: NONE,
	exec_datatable_sql: RUN_PREVIEW,

	// ── MCP ─────────────────────────────────────────────────────────────────
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

	// ── Authoring aids ──────────────────────────────────────────────────────
	// Ungated where the call really is a read the server serves anyone: a hub, npm or
	// resource-type query, or a schema built client-side. The prompt bullets naming them
	// are gated on `write_draft`, so a session that cannot author is never pointed at them.
	get_instructions: NONE,
	search_hub_scripts: NONE,
	search_npm_packages: NONE,
	search_resource_types: NONE,
	get_trigger_schema: NONE,
	get_schedule_schema: NONE,
	// Runs a query script (`getDbSchemas` → /jobs/run/preview), which jobs.rs refuses operators.
	get_db_schema: RUN_PREVIEW,
	// `folder` is one of the gated kinds: folders.rs `create_folder` runs
	// `check_deploy_rules`.
	create_folder: ['deploy'],
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
	// (drafts.rs) so that a user who has LOST write access can still clean up. Rebasing
	// is not exempt — it writes a fresh draft.
	discard_local_draft: NONE,
	rebase_draft: WRITE_DRAFT,

	// ── Deployed-object mutations ───────────────────────────────────────────
	// Neither requires `deploy`: both take the kind as an argument, so no workspace
	// refuses them outright. They part ways on their input — deploying persists a draft,
	// and nothing but `write_draft` makes one, while deleting acts on what is deployed.
	deploy_workspace_item: WRITE_DRAFT,
	delete_workspace_item: NONE,

	// Ungated, unlike the preview runs below: these execute the DEPLOYED item under
	// the user's own permissions, which is the one run an operator's token allows.
	run_script: NONE,
	run_flow: NONE,

	// ── Preview execution ───────────────────────────────────────────────────
	test_run_script: RUN_PREVIEW,
	test_run_flow: RUN_PREVIEW,
	test_run_step: RUN_PREVIEW,
	// Reaches apps.rs `execute_component` rather than jobs.rs, but that handler
	// refuses operators too once `force_viewer_static_fields` marks it a preview.
	test_run_app_runnable: RUN_PREVIEW,

	// ── Pipeline editor ─────────────────────────────────────────────────────
	get_pipeline_graph: NONE,
	read_pipeline_node: NONE,
	build_pipeline_node: WRITE_DRAFT,
	edit_pipeline_node: WRITE_DRAFT,
	remove_pipeline_node: WRITE_DRAFT,
	test_pipeline_node: RUN_PREVIEW
}

export function sessionToolAllowed(name: string, access: SessionAccess): boolean {
	const requires = SESSION_TOOL_POLICIES[name]
	// Fails closed, so a tool that ships without a policy disappears from restricted
	// sessions rather than leaking into them.
	if (!requires) return false
	return requires.every((c) => access.has(c))
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
