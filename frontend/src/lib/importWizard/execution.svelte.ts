import { WorkspaceService } from '$lib/gen'
import { switchWorkspace } from '$lib/storeUtils'
import { userStore, workspaceStore } from '$lib/stores'
import { getUserExt } from '$lib/user'
import { get } from 'svelte/store'
import { enterNewWorkspace, refreshWorkspaceList } from '$lib/workspaceCreation'
import {
	installProject,
	type InstallResult
} from '$lib/components/workspaceSettings/projectInstall'
import type {
	ProjectExport,
	ProjectMigration
} from '$lib/components/workspaceSettings/projectBundle'
import { planWorkspaceId, type ImportPlan } from './plan'
import { clearParkedImport, parkImport, resumableImport } from './parking'

/**
 * The only thing in the wizard that changes anything. It takes a finished plan and
 * runs it as an ordered, observable list of tasks, so the last step can show what
 * is happening and exactly where it stopped.
 *
 * Everything the run needs from the outside — reviewing data table migrations,
 * the EE licence — is injected, so the wizard's UI decisions stay in the wizard
 * and this file stays testable without a browser.
 */

/**
 * Whether an import is mid-flight, readable by anything that can navigate away
 * from it. The run outlives no component: leaving the last step unmounts the
 * migration review the executor is awaiting, so a run in progress has to block
 * the stepper and the browser rather than be silently detached.
 */
const runState = $state({ active: false })
export function importIsRunning(): boolean {
	return runState.active
}

export type TaskStatus = 'pending' | 'running' | 'done' | 'failed' | 'skipped'

export interface TaskView {
	key: string
	label: string
	status: TaskStatus
	detail?: string
}

/**
 * What a plan will do, as the same task list the run reports against. Exported so the
 * last step can show it before the run starts: the checklist is what the step says it
 * is going to do, and the run then fills in the same rows rather than replacing them.
 *
 * Derived from the plan alone — no network — so it is safe to call while rendering.
 */
export function plannedTasks(plan: ImportPlan): TaskView[] {
	const d = plan.destination
	const tasks: TaskView[] = []
	if (d?.kind === 'new') {
		tasks.push({ key: 'create', label: `Create workspace ${d.id}`, status: 'pending' })
	}
	tasks.push({ key: 'fetch', label: 'Fetch the project from the hub', status: 'pending' })
	// The destination rides on this row when nothing creates it, so the list still says
	// where the items are going in the existing-workspace case.
	tasks.push({
		key: 'import',
		label: d?.kind === 'existing' ? `Import the items into ${d.workspaceId}` : 'Import the items',
		status: 'pending'
	})
	return tasks
}

export interface ExecutionDeps {
	/**
	 * Chooses which data table migrations to run. Returns the migrations to apply,
	 * or null to abort the whole import (the user backed out at the warning).
	 */
	reviewMigrations: (
		workspace: string,
		migrations: ProjectMigration[]
	) => Promise<ProjectMigration[] | null>
	hasEeLicense: boolean
}

export class ImportExecution {
	#plan: ImportPlan
	#deps: ExecutionDeps

	/** Carried between attempts so a retry does not redo finished work. */
	// The hub's summary endpoint counts scripts/flows/apps/resources; only the export
	// carries triggers and data table migrations. Surfaced so the last step can say
	// what it is about to create — the warning below it talks about triggers, and the
	// page this replaced did show both.
	#export = $state<ProjectExport | undefined>(undefined)

	/**
	 * Data tables the project's migrations target. The wizard compares these with the
	 * destination's configured tables to decide whether a setup step is needed —
	 * `installProject` skips a migration whose data table does not exist.
	 */
	get datatableNames(): string[] {
		const e = this.#export
		if (!e) return []
		return [
			...new Set(
				(e.migrations ?? [])
					.filter((m) => m.enabled && (m.sql ?? '').trim() !== '')
					.map((m) => m.datatable_name)
			)
		]
	}

	/**
	 * How many resources the project shipped. Every one arrives as an empty stub —
	 * the hub never publishes resource values — so a non-zero count means the setup
	 * step has something to offer.
	 */
	get resourceCount(): number {
		return this.#export?.resources?.length ?? 0
	}

	get extraCounts(): { triggers: number; migrations: number } | undefined {
		const e = this.#export
		if (!e) return undefined
		return {
			triggers: e.triggers?.length ?? 0,
			migrations: (e.migrations ?? []).filter((m) => m.enabled && (m.sql ?? '').trim() !== '')
				.length
		}
	}
	// $state, not a plain field: the UI offers to delete the workspace this run
	// created, and a plain field would never re-render that button.
	#workspaceCreated = $state(false)

	tasks = $state<TaskView[]>([])
	results = $state<InstallResult[]>([])
	running = $state(false)
	/** Set when a run stopped early; cleared when a retry starts. */
	error = $state<string | undefined>(undefined)
	done = $state(false)

	// Where the app pointed before this run switched away from it, so undoing the run
	// can put it back. Captured at construction rather than at switch time: by then
	// `$workspaceStore` already holds the workspace being entered.
	#priorWorkspace = get(workspaceStore)

	/**
	 * A workspace this plan created before the page was reloaded. Only ever true for a run
	 * whose create already succeeded: `resumableImport` requires the parked project *and*
	 * workspace to be this plan's, so an entry left by another import cannot make this run
	 * skip a create it has not done.
	 */
	#resumed: boolean

	constructor(plan: ImportPlan, deps: ExecutionDeps) {
		this.#plan = plan
		this.#deps = deps
		const d = plan.destination
		this.#resumed = d?.kind === 'new' && resumableImport(plan.slug, d.id)
		this.tasks = this.#initialTasks()
	}

	get workspaceId(): string | undefined {
		return planWorkspaceId(this.#plan)
	}

	/**
	 * True once this run created a workspace — the only case where deleting is ours to offer.
	 *
	 * Deliberately not satisfied by `#resumed`. A parked entry is enough to skip a create,
	 * because entering the wrong workspace is recoverable; it is not enough to delete one,
	 * because that is not. Verifying would need a discriminator to compare the live
	 * workspace against, and a workspace has none — no `created_at`, nothing that moves
	 * when someone else writes — so a parked id could name a workspace another admin made
	 * at that id after ours was removed. A resumed run therefore finishes the import and
	 * leaves the undo to the run that actually did the creating.
	 */
	get createdWorkspace(): boolean {
		return this.#workspaceCreated
	}

	get failedCount(): number {
		return this.results.filter((r) => !r.ok).length
	}

	/** `installProject` reports migrations through the same channel as items, tagged by this
	 *  prefix. Split so the import row counts what it imported and the migrate row counts
	 *  what it migrated — one failure should not be attributed to both. */
	static readonly MIGRATION_PREFIX = 'data table: '
	get itemResults(): InstallResult[] {
		return this.results.filter((r) => !r.path.startsWith(ImportExecution.MIGRATION_PREFIX))
	}
	get migrationResults(): InstallResult[] {
		return this.results.filter((r) => r.path.startsWith(ImportExecution.MIGRATION_PREFIX))
	}

	#initialTasks(): TaskView[] {
		return plannedTasks(this.#plan)
	}

	#set(key: string, status: TaskStatus, detail?: string) {
		this.tasks = this.tasks.map((t) => (t.key === key ? { ...t, status, detail } : t))
	}

	/**
	 * Runs every task that has not already succeeded. Safe to call again after a
	 * failure: a created workspace and a fetched export are reused rather than
	 * repeated. The granularity is the task, not the item — a retry re-runs
	 * `installProject` over the whole bundle, which is idempotent per item but does
	 * not skip the ones that already landed.
	 */
	async run(): Promise<void> {
		if (this.running) return
		this.running = true
		runState.active = true
		this.error = undefined
		try {
			const workspace = await this.#ensureWorkspace()
			if (!workspace) return
			const exportData = await this.#ensureExport(workspace)
			if (!exportData) return
			await this.#import(workspace, exportData)
		} finally {
			this.running = false
			runState.active = false
		}
	}

	/**
	 * The folder is the one part of the plan still editable on the last step, so a
	 * retry after changing it must import where the field now says — not where the
	 * first attempt was told to.
	 */
	setFolder(folder: string) {
		this.#plan = { ...this.#plan, folder }
	}

	async #ensureWorkspace(): Promise<string | undefined> {
		const d = this.#plan.destination
		if (!d) {
			this.error = 'No destination'
			return undefined
		}
		if (d.kind === 'existing') {
			if (!d.workspaceId) {
				this.error = 'No destination workspace'
				return undefined
			}
			switchWorkspace(d.workspaceId)
			await this.#adoptUser(d.workspaceId)
			return d.workspaceId
		}
		// Keyed on the workspace existing rather than on the task being green: a retry
		// after entering it failed must not run the create again, which would only
		// report the id as taken by the workspace this run just made. `#resumed` covers
		// the same ground across a reload, where the field starts false again.
		if (!this.#workspaceCreated && !this.#resumed) {
			this.#set('create', 'running')
			try {
				await WorkspaceService.createWorkspace({
					requestBody: { id: d.id, name: d.name, username: d.username }
				})
			} catch (e: any) {
				const detail = e?.body?.toString?.() ?? String(e)
				this.#set('create', 'failed', detail)
				this.error = `Could not create the workspace: ${detail}`
				return undefined
			}
			this.#workspaceCreated = true
			// From here a reload can no longer tell that this id is ours, so record it
			// before anything else can fail.
			parkImport({ slug: this.#plan.slug, workspaceId: d.id })
		}
		try {
			await enterNewWorkspace(d.id)
			await this.#adoptUser(d.id)
		} catch (e: any) {
			const detail = e?.body?.toString?.() ?? String(e)
			this.#set('create', 'failed', `created, but could not be entered: ${detail}`)
			this.error = `Created ${d.id}, but could not enter it: ${detail}`
			return undefined
		}
		this.#set('create', 'done')
		return d.id
	}

	async #ensureExport(workspace: string): Promise<ProjectExport | undefined> {
		if (this.#export) {
			this.#set('fetch', 'done')
			return this.#export
		}
		this.#set('fetch', 'running')
		try {
			// Workspace-scoped on purpose: this is the same proxy the rest of the app
			// uses, so a private hub reachable only from the server still works.
			const res = await fetch(
				`/api/w/${encodeURIComponent(workspace)}/hub/projects/${encodeURIComponent(this.#plan.slug)}/export`,
				{ credentials: 'include', headers: { accept: 'application/json' } }
			)
			const text = await res.text()
			if (!res.ok) throw new Error(`export ${res.status}: ${text}`)
			this.#export = JSON.parse(text) as ProjectExport
			this.#set('fetch', 'done', `${itemCount(this.#export)} items`)
			return this.#export
		} catch (e: any) {
			const detail = e?.message ?? String(e)
			this.#set('fetch', 'failed', detail)
			this.error = `Could not read the project: ${detail}`
			return undefined
		}
	}

	async #import(workspace: string, exportData: ProjectExport): Promise<void> {
		this.#set('import', 'running')
		const folder = this.#plan.folder?.trim() || exportData.project.slug

		let migrations: ProjectMigration[] | null
		try {
			migrations = await this.#deps.reviewMigrations(workspace, exportData.migrations ?? [])
		} catch (e: any) {
			this.#set('import', 'failed', String(e))
			this.error = `Could not plan the data table migrations: ${e}`
			return
		}
		if (migrations === null) {
			// The user backed out at the missing-data-table warning.
			this.#set('import', 'skipped', 'cancelled at the data table warning')
			this.error = 'Import cancelled.'
			return
		}

		// Appended only once the review has settled: until then nothing knows whether any
		// migration is runnable here, and a row that might not apply is worse than none.
		if (migrations.length && !this.tasks.some((t) => t.key === 'migrate')) {
			const n = migrations.length
			this.tasks = [
				...this.tasks,
				{
					key: 'migrate',
					label: `Run ${n} data table migration${n === 1 ? '' : 's'}`,
					status: 'pending'
				}
			]
		}

		this.results = []
		try {
			await installProject({
				workspace,
				exportData,
				folder,
				migrations,
				hasEeLicense: this.#deps.hasEeLicense,
				onResult: (r) => (this.results = [...this.results, r]),
				onMigrationsStart: () => this.#set('migrate', 'running')
			})
		} catch (e: any) {
			this.#set('import', 'failed', String(e))
			this.error = `The import stopped: ${e}`
			return
		}

		const items = this.itemResults
		const failed = items.filter((r) => !r.ok).length
		this.#set(
			'import',
			failed > 0 ? 'failed' : 'done',
			failed > 0 ? `${items.length - failed} of ${items.length} imported` : `${items.length} items`
		)

		const migrated = this.migrationResults
		if (migrated.length) {
			const badly = migrated.filter((r) => !r.ok)
			this.#set(
				'migrate',
				badly.length ? 'failed' : 'done',
				badly.length ? badly.map((r) => r.error).join('; ') : undefined
			)
		}
		// A partial import is finished, not broken: the items that landed are real,
		// and the failures are listed. Only a hard stop leaves `done` false.
		this.done = true
		// Nothing left to resume. A later import of the same project must reach its
		// create rather than adopt this one.
		clearParkedImport()
		if (failed > 0) this.error = `${failed} item${failed === 1 ? '' : 's'} failed to import.`
	}

	/**
	 * Load the membership for the workspace this run just entered.
	 *
	 * The wizard's page is reparented out of `(logged)`, so it never gets that
	 * layout's `getUserExt` call and `$userStore` stays undefined. Anything deciding
	 * what the user may do then reads "no user" and refuses: `canWrite` returns false
	 * without one, which renders every field of the resource editor disabled — the
	 * setup step could show the credentials to fill and then not let anyone fill them.
	 */
	async #adoptUser(workspace: string): Promise<void> {
		try {
			userStore.set(await getUserExt(workspace))
		} catch {
			// Leave it unset; the step degrades to read-only rather than failing the run.
		}
	}

	/** Undoes the one thing this run created, when the user asks for it. */
	async deleteCreatedWorkspace(): Promise<void> {
		const d = this.#plan.destination
		if (!this.#workspaceCreated || d?.kind !== 'new') return
		await WorkspaceService.deleteWorkspace({ workspace: d.id })
		// Leave the app pointing somewhere that exists. The run switched into the
		// workspace it created; without this the store keeps the deleted id, the
		// layout persists it to local/sessionStorage, and the next full page load
		// fails `getUserExt` and logs the user out.
		switchWorkspace(this.#priorWorkspace)
		await refreshWorkspaceList()
		this.#workspaceCreated = false
		// The id is free again, so a retry has to create it rather than adopt it.
		clearParkedImport()
		this.#set('create', 'pending')
		this.done = false
		this.results = []
	}
}

function itemCount(e: ProjectExport): number {
	return e.scripts.length + e.flows.length + e.apps.length + e.resources.length
}
