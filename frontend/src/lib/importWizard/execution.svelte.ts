import { UserService, WorkspaceService } from '$lib/gen'
import { switchWorkspace } from '$lib/storeUtils'
import { userStore, workspaceStore } from '$lib/stores'
import { getUserExt } from '$lib/user'
import { get } from 'svelte/store'
import { enterNewWorkspace, refreshWorkspaceList } from '$lib/workspaceCreation'
import {
	installProject,
	type InstallResult
} from '$lib/components/workspaceSettings/projectInstall'
import {
	projectReferencesResource,
	type ProjectExport,
	type ProjectMigration
} from '$lib/components/workspaceSettings/projectBundle'
import { planWorkspaceId, type ImportPlan } from './plan'
import { probeImportedPaths, probeWorkspace } from './probe'

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
	 * How many of the project's resources the setup step will ask about. Every resource
	 * arrives as an empty stub — the hub never publishes resource values — but only the ones
	 * something in the project points at have to hold a credential for it to work, and those
	 * are the ones the step lists. Counting all of them here would offer a fourth step that
	 * then has nothing on it.
	 */
	get resourceCount(): number {
		const e = this.#export
		if (!e) return 0
		return (e.resources ?? []).filter((r) => projectReferencesResource(e, String(r.path))).length
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

	constructor(plan: ImportPlan, deps: ExecutionDeps) {
		this.#plan = plan
		this.#deps = deps
		this.tasks = this.#initialTasks()
	}

	/**
	 * Identifies the plan this run belongs to — destination and project, not the folder,
	 * which stays editable on the last step and is pushed onto the run instead. A caller
	 * handing a run back after a remount compares this against the plan it is rendering;
	 * computing the tag from *that* plan would make the check pass by construction.
	 */
	get planTag(): string {
		return JSON.stringify(this.#plan.destination) + this.#plan.slug
	}

	get workspaceId(): string | undefined {
		return planWorkspaceId(this.#plan)
	}

	/**
	 * True once this run created a workspace — the only case where deleting is ours to offer.
	 *
	 * Deliberately not satisfied by having *adopted* one. `workspace.owner` is enough to know
	 * a create can be skipped — the id is one this user made — but not enough to offer to
	 * delete it, because `owner` is an identity, not a run: a second import by the same person
	 * into the same id looks identical. Skipping a create wrongly is recoverable; deleting a
	 * workspace is not, so an adopted run finishes the import and leaves the undo to the run
	 * that did the creating.
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
	 * Set when the user confirms leaving mid-run. Nothing here can abort a request already
	 * in flight — `installProject` takes no signal — so this stops the run at the next phase
	 * boundary instead, which is as far as "stops where it is" can honestly go.
	 *
	 * The workspace it created stays, and the run stays resumable: coming back to the link
	 * re-probes the instance, finds the workspace, and carries on rather than trying to create
	 * it a second time.
	 */
	#abandoned = false

	/** The user has left. Stop at the next phase boundary and leave the run resumable. */
	abandon() {
		this.#abandoned = true
	}

	/**
	 * Runs every task that has not already succeeded. Safe to call again after a failure: the
	 * destination is asked what it already holds, so a workspace that exists is entered rather
	 * than recreated and an item that landed is skipped rather than rewritten. What a second
	 * run costs is the reads, not the writes.
	 */
	async run(): Promise<void> {
		if (this.running) return
		this.#abandoned = false
		this.running = true
		runState.active = true
		this.error = undefined
		try {
			const workspace = await this.#ensureWorkspace()
			if (!workspace || this.#abandoned) return
			const exportData = await this.#ensureExport(workspace)
			if (!exportData || this.#abandoned) return
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
		// Asked of the instance, not remembered. A retry after entering it failed must not
		// run the create again — that would only report the id as taken by the workspace this
		// run just made — and after a reload the field is false again while the workspace is
		// still there. `ours` is what makes adopting it safe: an id that exists but belongs to
		// someone else is not this run's work, and importing into it would be importing into
		// a stranger's workspace.
		const already = await probeWorkspace(d.id, await this.#email())
		if (!this.#workspaceCreated && !(already.exists && already.ours)) {
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

	/**
	 * Leave the checklist saying what actually happened, from wherever the run stopped.
	 *
	 * Reached from every point after `import` goes `running`, so nothing is left spinning on a
	 * run that has ended. A partial import is failed rather than done: calling it done reports
	 * a clean import over items that never started, and the resumed step offers Continue where
	 * it should offer Retry.
	 */
	#settleAbandoned() {
		const landed = this.itemResults.length
		this.#set('import', 'failed', `stopped after ${landed} item${landed === 1 ? '' : 's'}`)
		// The migrate row is appended once the review settles and set running by
		// `onMigrationsStart`. Stopping before its loop leaves it spinning forever, which
		// reads as work still in progress on a run that has stopped.
		if (this.tasks.some((t) => t.key === 'migrate' && t.status === 'running')) {
			this.#set('migrate', 'pending')
		}
		this.error = 'Import stopped. Retry to import what is left.'
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
		// Asked every run, not only on a retry: the destination may be a workspace that already
		// holds some of these paths, and a run interrupted halfway is indistinguishable from
		// one that never started. On a workspace this run just created the answer is empty and
		// nothing is skipped.
		const alreadyPresent = await probeImportedPaths(workspace, folder, {
			triggers: exportData.triggers.length > 0,
			hasEeLicense: this.#deps.hasEeLicense
		})
		// Settled, not just returned: `import` has been `running` since before the probe, and
		// leaving it there shows a spinner on a run that has stopped, next to a Retry button.
		if (this.#abandoned) {
			this.#settleAbandoned()
			return
		}
		try {
			await installProject({
				alreadyPresent,
				workspace,
				exportData,
				folder,
				migrations,
				hasEeLicense: this.#deps.hasEeLicense,
				onResult: (r) => (this.results = [...this.results, r]),
				onMigrationsStart: () => this.#set('migrate', 'running'),
				// Checked before every write, so leaving mid-run stops the remaining items
				// rather than only the phases. What already landed stays and is listed.
				stopped: () => this.#abandoned
			})
		} catch (e: any) {
			this.#set('import', 'failed', String(e))
			this.error = `The import stopped: ${e}`
			return
		}

		// `installProject` returns early when `stopped` goes true, and it returns the same way
		// it does on success — so the tail has to ask why. An abandoned run has written only
		// what it got through; calling that `done` reports a clean import over items that
		// never started, and the resumed step would offer Continue instead of Retry.
		if (this.#abandoned) {
			this.#settleAbandoned()
			return
		}

		const items = this.itemResults
		const failed = items.filter((r) => !r.ok).length
		const skipped = items.filter((r) => r.skipped).length
		// Three outcomes, so the row says which: written, left alone because it was already
		// there, and failed. Rolling the second into the first would report an import that
		// did not happen.
		const wrote = items.length - failed - skipped
		const parts: string[] = []
		if (wrote > 0 || (failed === 0 && skipped === 0)) parts.push(`${wrote} imported`)
		if (skipped > 0) parts.push(`${skipped} already there`)
		if (failed > 0) parts.push(`${failed} failed`)
		this.#set('import', failed > 0 ? 'failed' : 'done', parts.join(', '))

		const migrated = this.migrationResults
		const badMigrations = migrated.filter((r) => !r.ok).length
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
		// Both kinds of failure, because `error` is what offers Retry. A migration that fails
		// against an existing data table is as retryable as a failed item; leaving it out here
		// would present the run as a clean finish with no way to run it again.
		const problems: string[] = []
		if (failed > 0) problems.push(`${failed} item${failed === 1 ? '' : 's'} failed to import`)
		if (badMigrations > 0) {
			problems.push(`${badMigrations} data table migration${badMigrations === 1 ? '' : 's'} failed`)
		}
		if (problems.length) this.error = `${problems.join(', ')}.`
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

	/**
	 * Who we are, for the ownership check. `$userStore` is workspace-scoped and this page is
	 * reparented out of `(logged)`, so it is unset until a run adopts one — `globalWhoami` is
	 * the identity that exists before any workspace does.
	 */
	async #email(): Promise<string | undefined> {
		const known = get(userStore)?.email
		if (known) return known
		try {
			return (await UserService.globalWhoami()).email
		} catch {
			return undefined
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
		// The id is free again; the next `#ensureWorkspace` asks the instance and finds it
		// gone, so a retry creates rather than adopts.
		this.#set('create', 'pending')
		this.done = false
		this.results = []
	}
}

function itemCount(e: ProjectExport): number {
	return e.scripts.length + e.flows.length + e.apps.length + e.resources.length
}
