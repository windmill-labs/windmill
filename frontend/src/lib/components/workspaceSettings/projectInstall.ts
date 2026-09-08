// Imports a Hub project export into a workspace: one importer per item kind,
// each item reported individually so one bad item never aborts the rest.
// UI-free — the install page owns folder choice and migration review.

import {
	AppService,
	FlowService,
	FolderService,
	ResourceService,
	ScriptService,
	VariableService,
	WorkspaceService
} from '$lib/gen'
import {
	TRIGGER_KINDS,
	createWorkspaceTriggerDisabled,
	triggerHandlerRefs,
	type WorkspaceTrigger,
	type WorkspaceTriggerKind
} from '../triggers/workspaceTriggersList'
import { updatePolicy } from '$lib/components/apps/editor/appPolicy'
import { updateRawAppPolicy } from '$lib/sharedUtils'
import { apiErrorMessage as errorMessage } from '$lib/utils'
import type { App } from '$lib/components/apps/types'
import { runScriptAndPollResult } from '$lib/components/jobs/utils'
import { writingJobOptions } from '$lib/components/jobs/writingJob'
import {
	classifyPath,
	collectExportVarPaths,
	extractAppRefs,
	extractFlowRefs,
	extractRawAppRefs,
	extractScriptRefs,
	extractTriggerConfigResourceRefs,
	extractVarRefsFromValue,
	retargetProjectExport,
	type ExportItem,
	type ProjectExport,
	type ProjectMigration,
	type Ref
} from './projectBundle'

export interface InstallResult {
	path: string
	ok: boolean
	error?: string
	/**
	 * Already in the destination, so nothing was written. Not a failure and not an import —
	 * reporting it as either would be a lie, and the difference is what a retry is for.
	 */
	skipped?: boolean
}

// Guarding an item's own path is not enough: the `$res:`/script/flow refs baked
// into its content are live bindings the backend acts on. A well-formed export
// relocates them all into f/<folder>/ (hub/ script refs stay external); anything
// else points a runnable at an existing asset in another namespace, so refuse the
// item rather than bind it there. Resources are never hub-hosted, so a hub/ path
// there is not a valid escape hatch. Mirrors the trigger-config containment.
export function refContainmentViolation(refs: Ref[], folder: string): string | undefined {
	for (const r of refs) {
		const cls = classifyPath(r.path, folder)
		if (cls === 'internal') continue
		if (cls === 'hub' && r.kind !== 'resource') continue
		return `reference '${r.path}' escapes the target folder f/${folder}/ — skipped`
	}
	return undefined
}

// `$var:`/`$jsonvar:` references (in flow static inputs, flow_env, app runnable
// inputs, trigger config) are resolved at runtime under the imported runnable's
// permissions and are never hub-hosted. Retargeting relocates a project's own refs
// into the target folder; anything still outside it points at another namespace, so
// reject those. Takes the parsed value so inline code carrying a literal is ignored.
export function varContainmentViolation(value: any, folder: string): string | undefined {
	for (const p of extractVarRefsFromValue(value)) {
		if (classifyPath(p, folder) !== 'internal') {
			return `variable '${p}' escapes the target folder f/${folder}/ — skipped`
		}
	}
	return undefined
}

// Recompute an app's execution policy from its (retargeted) value, mirroring
// what the editor does on deploy. `triggerables_v2` is keyed by
// `<component>:rawscript/<sha256(inline content)>`; retargeting rewrites that
// content, so a copied or empty policy would leave every inline runnable
// "forbidden by policy" at runtime. Default to publisher (auth required).
async function computeAppPolicy(value: any): Promise<any> {
	const policy = (await updatePolicy(value as App, undefined)) as any
	if (!policy.execution_mode) policy.execution_mode = 'publisher'
	return policy
}
async function computeRawAppPolicy(runnables: Record<string, any>): Promise<any> {
	const policy = (await updateRawAppPolicy(runnables, undefined)) as any
	if (!policy.execution_mode) policy.execution_mode = 'publisher'
	return policy
}

function importScript(workspace: string, s: ExportItem): Promise<unknown> {
	return ScriptService.createScript({
		workspace,
		requestBody: {
			path: s.path,
			summary: s.summary ?? '',
			description: s.description ?? '',
			content: s.content ?? '',
			language: s.language,
			schema: s.schema ?? undefined,
			kind: s.kind ?? 'script',
			lock: s.lockfile ?? undefined
		}
	})
}

function importFlow(workspace: string, f: ExportItem): Promise<unknown> {
	return FlowService.createFlow({
		workspace,
		requestBody: {
			path: f.path,
			summary: f.summary ?? '',
			description: f.description ?? '',
			value: f.value,
			schema: f.schema ?? undefined
		}
	})
}

// Stubs only: never overwrite an existing resource's value (updateIfExists
// stays false so a path collision is reported as a failed item instead).
function importResourceStub(workspace: string, r: ExportItem): Promise<unknown> {
	return ResourceService.createResource({
		workspace,
		updateIfExists: false,
		requestBody: {
			path: r.path,
			resource_type: r.resource_type,
			value: {},
			description: 'Imported stub — fill in the value.'
		}
	})
}

// Variables hold secrets/config, so their values are never shipped. Create an empty
// secret placeholder for a project variable the importer must fill, mirroring the
// resource stubs. Conflict-safe: an already-present variable (the importer filled it,
// or a re-import) is left untouched rather than clobbered.
async function importVariablePlaceholder(workspace: string, path: string): Promise<void> {
	if (await VariableService.existsVariable({ workspace, path })) return
	await VariableService.createVariable({
		workspace,
		requestBody: {
			path,
			value: '',
			is_secret: true,
			description: 'Imported placeholder — fill in the value.'
		}
	})
}

async function importApp(workspace: string, a: ExportItem): Promise<unknown> {
	if (a.app_type === 'raw') {
		let parsed: any
		try {
			parsed = JSON.parse(a.value?.raw ?? '{}')
		} catch (e: any) {
			throw new Error(`invalid raw app bundle: ${e?.message ?? String(e)}`)
		}
		const files = { ...(parsed.files ?? {}) }
		const js = files['/bundle.js'] ?? ''
		const css = files['/bundle.css'] ?? ''
		delete files['/bundle.js']
		delete files['/bundle.css']
		const runnables = parsed.runnables ?? {}
		return AppService.createAppRaw({
			workspace,
			formData: {
				app: {
					path: a.path,
					summary: a.summary ?? '',
					value: {
						files,
						runnables,
						// Keep the full-code app's explicit data table declaration.
						...(parsed.data !== undefined ? { data: parsed.data } : {}),
						...(parsed.datatables !== undefined ? { datatables: parsed.datatables } : {})
					},
					policy: await computeRawAppPolicy(runnables)
				},
				js,
				css
			}
		})
	}
	return AppService.createApp({
		workspace,
		requestBody: {
			path: a.path,
			summary: a.summary ?? '',
			value: a.value,
			policy: await computeAppPolicy(a.value)
		}
	})
}

// Apply one migration to the target data table. If the data table opted into
// migrations, record it (datatable_migrations + _wm_migrations, run only this
// version); otherwise run the SQL once as a preview job (unrecorded).
//
// Exported because the import can leave migrations unapplied: a data table the
// project needs may not be configured in the destination yet, and the wizard's setup
// step runs them once it is.
export async function applyOneMigration(
	workspace: string,
	projectSlug: string,
	m: ProjectMigration
): Promise<void> {
	let recorded = false
	try {
		const status = await WorkspaceService.getDatatableMigrationsStatus({
			workspace,
			datatableName: m.datatable_name
		})
		recorded = !!status.enabled
	} catch {}

	if (recorded) {
		// Record the shipped down migration (DROP the created tables) so it can be
		// rolled back.
		const codeDown = (m.sql_down ?? '').trim()
		const created = await WorkspaceService.createDatatableMigration({
			workspace,
			datatableName: m.datatable_name,
			requestBody: {
				name: `hub_import_${projectSlug}`,
				code_up: m.sql,
				code_down: codeDown || undefined
			}
		})
		await WorkspaceService.runDatatableMigrations({
			workspace,
			datatableName: m.datatable_name,
			only: created.timestamp
		})
	} else {
		await runScriptAndPollResult(
			{
				workspace,
				requestBody: {
					language: 'postgresql',
					content: m.sql,
					args: { database: `datatable://${m.datatable_name}` }
				}
			},
			writingJobOptions
		)
	}
}

/**
 * Install a project export into `workspace` under `f/<folder>/`: create the
 * folder, retarget every item, import kind by kind, then apply the (already
 * reviewed) migrations. Each item's outcome is reported through `onResult`;
 * failures never abort the remaining items.
 */
/**
 * The kinds an import writes that carry a path and can therefore already be there.
 *
 * Triggers carry their own kind too (`trigger:schedule`, `trigger:http`, … — the values of
 * `WorkspaceTriggerKind`): each trigger kind is a separate table keyed on
 * `(path, workspace_id)`, so one workspace can hold a schedule and an HTTP trigger both
 * called `f/cal/sync`. Flattening them to `trigger` would let whichever exists answer for
 * the other.
 */
export type ImportedKind =
	| 'script'
	| 'flow'
	| 'app'
	| 'resource'
	| `trigger:${string}`

/**
 * The key `alreadyPresent` is built and read with. Kind and path together, because the kinds
 * share one path namespace and a bare path cannot say which of them is already there.
 */
export function presenceKey(kind: ImportedKind, path: string): string {
	return `${kind}:${path}`
}

export async function installProject(args: {
	workspace: string
	exportData: ProjectExport
	folder: string
	migrations: ProjectMigration[]
	/** Called once, before the reviewed migrations are applied, when there are any. Lets a
	 *  caller show them as their own step rather than folding them into the item import. */
	onMigrationsStart?: () => void
	/**
	 * Asked before each write. Returning true stops the run where it is — the writes already
	 * made stay, the rest never start. Nothing here can cancel a request already in flight,
	 * so this is the granularity available without threading an `AbortSignal` through every
	 * service call: the import wizard uses it when the user confirms leaving mid-run.
	 */
	stopped?: () => boolean
	/**
	 * What is already in the destination, as `presenceKey` keys — so a retry writes only what
	 * is missing instead of replaying the bundle into a wall of "already exists". Built from
	 * retargeted paths, because that is what these items will actually be called.
	 *
	 * Keyed by kind and not by path alone: the five kinds share one `f/<folder>/` namespace, so
	 * a trigger and a script may legitimately both be called `f/cal/sync`. A flat path set
	 * would let either one mask the other and silently skip an item that was never imported.
	 *
	 * Never a way to *replace* anything: an item that is there is left exactly as it is,
	 * which is the same promise `updateIfExists: false` makes for a resource whose value
	 * someone has since filled in.
	 */
	alreadyPresent?: Set<string>
	hasEeLicense: boolean
	onResult: (r: InstallResult) => void
}): Promise<void> {
	const {
		workspace,
		exportData,
		folder,
		migrations,
		hasEeLicense,
		onResult,
		onMigrationsStart,
		stopped,
		alreadyPresent
	} = args

	const record = (path: string, p: Promise<unknown>): Promise<void> =>
		p.then(
			() => onResult({ path, ok: true }),
			(e: any) => onResult({ path, ok: false, error: errorMessage(e) })
		)

	/** Every write goes through here, so one check covers items, variables and migrations. */
	const halted = () => stopped?.() === true

	/**
	 * True when the destination already has this path, so the write is not attempted. Reported
	 * rather than dropped: the checklist has to account for every item the project ships, and
	 * "already there" is a different thing from "imported".
	 */
	const present = (kind: ImportedKind, path: string): boolean => {
		if (!alreadyPresent?.has(presenceKey(kind, path))) return false
		onResult({ path, ok: true, skipped: true })
		return true
	}

	try {
		await FolderService.createFolder({ workspace, requestBody: { name: folder } })
	} catch {}

	const proj = retargetProjectExport(exportData, exportData.project.slug, folder)

	// The export is remote input: every path it wants to write must stay inside
	// the folder the user chose. Anything else (crafted export, or an export
	// whose items weren't relocated into f/<slug>/ at publish) is refused
	// per-item instead of being created in another namespace.
	const prefix = `f/${folder}/`
	const guard = (path: unknown, ...also: unknown[]): string | undefined => {
		for (const p of [path, ...also]) {
			if (typeof p !== 'string' || !p.startsWith(prefix)) {
				return `path '${String(p)}' escapes the target folder ${prefix} — skipped`
			}
		}
		return undefined
	}
	const checked = (path: unknown, run: () => Promise<unknown>, ...also: unknown[]) => {
		const violation = guard(path, ...also)
		return violation
			? record(String(path), Promise.reject(new Error(violation)))
			: record(String(path), run())
	}

	// `refs` catches structured runnable/`$res:` refs; `varValue` is the parsed item
	// walked for `$var:`/`$jsonvar:` argument refs (which the ref extractors miss).
	const checkedItem = (path: unknown, refs: Ref[], varValue: any, run: () => Promise<unknown>) => {
		const violation =
			guard(path) ??
			refContainmentViolation(refs, folder) ??
			varContainmentViolation(varValue, folder)
		return violation
			? record(String(path), Promise.reject(new Error(violation)))
			: record(String(path), run())
	}

	for (const s of proj.scripts) {
		if (halted()) return
		if (present('script', s.path)) continue
		// `$var:` is resolved in job args (flow inputs, schedule args, trigger config),
		// not in script source, so there is no variable arg to contain here.
		await checkedItem(s.path, extractScriptRefs(s.content ?? ''), undefined, () =>
			importScript(workspace, s)
		)
	}
	for (const f of proj.flows) {
		if (halted()) return
		if (present('flow', f.path)) continue
		await checkedItem(f.path, extractFlowRefs(f.value), f.value, () => importFlow(workspace, f))
	}
	for (const r of proj.resources) {
		if (halted()) return
		if (present('resource', r.path)) continue
		await checked(r.path, () => importResourceStub(workspace, r))
	}
	// Placeholders for the project's internal `$var:`/`$jsonvar:` refs (retargeted
	// into this folder). External refs are rejected per-item, so only stub in-folder
	// ones; guard again in case an out-of-folder ref slipped through retargeting.
	for (const p of collectExportVarPaths(proj)) {
		if (halted()) return
		if (!p.startsWith(prefix)) continue
		await record(`variable: ${p}`, importVariablePlaceholder(workspace, p))
	}
	for (const a of proj.apps) {
		if (halted()) return
		if (present('app', a.path)) continue
		const isRaw = a.app_type === 'raw'
		const refs = isRaw ? extractRawAppRefs(a.value?.raw ?? '') : extractAppRefs(a.value)
		// Raw apps hold their runnables in the `value.raw` JSON string; parse it so the
		// walk sees the same structure the backend resolves. Malformed raw fails at import.
		let varValue: any = a.value
		if (isRaw) {
			try {
				varValue = JSON.parse(a.value?.raw ?? '{}')
			} catch {
				varValue = undefined
			}
		}
		await checkedItem(a.path, refs, varValue, () => importApp(workspace, a))
	}
	// A trigger's config is a live binding, not inert content: resource fields,
	// handler runnables and $res: refs it names are acted on by the backend, so
	// every one must stay inside the chosen folder (handlers may also point at
	// hub/ scripts). Otherwise a crafted export could bind the trigger to
	// existing assets in another namespace.
	const triggerConfigViolation = (t: ExportItem): string | undefined => {
		const cfg = (t.config ?? {}) as Record<string, any>
		for (const r of triggerHandlerRefs({ kind: t.kind, config: cfg } as WorkspaceTrigger)) {
			if (!r.path.startsWith(prefix) && !r.path.startsWith('hub/')) {
				return `handler '${r.path}' escapes the target folder ${prefix} — skipped`
			}
		}
		const resourceRefs = new Set(extractTriggerConfigResourceRefs(cfg))
		const field = TRIGGER_KINDS[t.kind as WorkspaceTriggerKind]?.resourceField
		const fieldValue = field ? cfg[field] : undefined
		if (typeof fieldValue === 'string' && fieldValue !== '') resourceRefs.add(fieldValue)
		for (const p of resourceRefs) {
			if (!p.startsWith(prefix)) {
				return `resource '${p}' escapes the target folder ${prefix} — skipped`
			}
		}
		// Config fields (e.g. SQS queue_url) can carry `$var:`/`$jsonvar:` refs too.
		return varContainmentViolation(cfg, folder)
	}
	for (const t of proj.triggers) {
		if (halted()) return
		if (present(`trigger:${t.kind}`, String(t.path))) continue
		const violation = guard(t.path, t.runnable_path) ?? triggerConfigViolation(t)
		await record(
			String(t.path),
			violation
				? Promise.reject(new Error(violation))
				: createWorkspaceTriggerDisabled(
						workspace,
						{
							kind: t.kind,
							path: t.path,
							script_path: t.runnable_path,
							is_flow: t.runnable_kind === 'flow',
							summary: t.summary ?? null,
							config: t.config ?? null
						},
						{ hasEeLicense }
					)
		)
	}

	// Apply the reviewed data table migrations after items exist.
	if (halted()) return
	if (migrations.length) onMigrationsStart?.()
	for (const m of migrations) {
		if (halted()) return
		await record(
			`data table: ${m.datatable_name}`,
			applyOneMigration(workspace, exportData.project.slug, m)
		)
	}
}
