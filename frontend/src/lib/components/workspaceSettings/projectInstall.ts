// Imports a Hub project export into a workspace: one importer per item kind,
// each item reported individually so one bad item never aborts the rest.
// UI-free — the install page owns folder choice and migration review.

import {
	AppService,
	FlowService,
	FolderService,
	ResourceService,
	ScriptService,
	WorkspaceService
} from '$lib/gen'
import { createWorkspaceTriggerDisabled } from '../triggers/workspaceTriggersList'
import { updatePolicy } from '$lib/components/apps/editor/appPolicy'
import { updateRawAppPolicy } from '$lib/sharedUtils'
import type { App } from '$lib/components/apps/types'
import { runScriptAndPollResult } from '$lib/components/jobs/utils'
import {
	retargetProjectExport,
	type ExportItem,
	type ProjectExport,
	type ProjectMigration
} from './projectBundle'

export interface InstallResult {
	path: string
	ok: boolean
	error?: string
}

// Surface the backend's explanation: API errors carry the real message in
// `.body` (plain text for Windmill 4xx), while `.message` is the generic
// status text ("Bad Request"). Prefer the body so e.g. a path/route_path
// collision reads as the actual reason, not just "Bad Request".
function errorMessage(e: any): string {
	const body = e?.body
	if (typeof body === 'string' && body.trim() !== '') return body
	if (body && typeof body === 'object')
		return body.error?.message ?? body.message ?? JSON.stringify(body)
	return e?.message ?? String(e)
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
async function applyOneMigration(
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
		await runScriptAndPollResult({
			workspace,
			requestBody: {
				language: 'postgresql',
				content: m.sql,
				args: { database: `datatable://${m.datatable_name}` }
			}
		})
	}
}

/**
 * Install a project export into `workspace` under `f/<folder>/`: create the
 * folder, retarget every item, import kind by kind, then apply the (already
 * reviewed) migrations. Each item's outcome is reported through `onResult`;
 * failures never abort the remaining items.
 */
export async function installProject(args: {
	workspace: string
	exportData: ProjectExport
	folder: string
	migrations: ProjectMigration[]
	hasEeLicense: boolean
	onResult: (r: InstallResult) => void
}): Promise<void> {
	const { workspace, exportData, folder, migrations, hasEeLicense, onResult } = args

	const record = (path: string, p: Promise<unknown>): Promise<void> =>
		p.then(
			() => onResult({ path, ok: true }),
			(e: any) => onResult({ path, ok: false, error: errorMessage(e) })
		)

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

	for (const s of proj.scripts) {
		await checked(s.path, () => importScript(workspace, s))
	}
	for (const f of proj.flows) {
		await checked(f.path, () => importFlow(workspace, f))
	}
	for (const r of proj.resources) {
		await checked(r.path, () => importResourceStub(workspace, r))
	}
	for (const a of proj.apps) {
		await checked(a.path, () => importApp(workspace, a))
	}
	for (const t of proj.triggers) {
		await checked(
			t.path,
			() =>
				createWorkspaceTriggerDisabled(
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
				),
			t.runnable_path
		)
	}

	// Apply the reviewed data table migrations after items exist.
	for (const m of migrations) {
		await record(
			`data table: ${m.datatable_name}`,
			applyOneMigration(workspace, exportData.project.slug, m)
		)
	}
}
