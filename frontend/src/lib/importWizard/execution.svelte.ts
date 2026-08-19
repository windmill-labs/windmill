import { WorkspaceService } from '$lib/gen'
import { switchWorkspace } from '$lib/storeUtils'
import { workspaceStore } from '$lib/stores'
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

	get workspaceId(): string | undefined {
		return planWorkspaceId(this.#plan)
	}

	/** True once this run created a workspace — the only case where deleting is ours to offer. */
	get createdWorkspace(): boolean {
		return this.#workspaceCreated
	}

	get failedCount(): number {
		return this.results.filter((r) => !r.ok).length
	}

	#initialTasks(): TaskView[] {
		const d = this.#plan.destination
		const tasks: TaskView[] = []
		if (d?.kind === 'new') {
			tasks.push({ key: 'create', label: `Create workspace ${d.id}`, status: 'pending' })
		}
		tasks.push({ key: 'fetch', label: 'Fetch the project from the hub', status: 'pending' })
		tasks.push({ key: 'import', label: 'Import the items', status: 'pending' })
		return tasks
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
			return d.workspaceId
		}
		// Keyed on the workspace existing rather than on the task being green: a retry
		// after entering it failed must not run the create again, which would only
		// report the id as taken by the workspace this run just made.
		if (!this.#workspaceCreated) {
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

		this.results = []
		try {
			await installProject({
				workspace,
				exportData,
				folder,
				migrations,
				hasEeLicense: this.#deps.hasEeLicense,
				onResult: (r) => (this.results = [...this.results, r])
			})
		} catch (e: any) {
			this.#set('import', 'failed', String(e))
			this.error = `The import stopped: ${e}`
			return
		}

		const failed = this.failedCount
		this.#set(
			'import',
			failed > 0 ? 'failed' : 'done',
			failed > 0
				? `${this.results.length - failed} of ${this.results.length} imported`
				: `${this.results.length} items`
		)
		// A partial import is finished, not broken: the items that landed are real,
		// and the failures are listed. Only a hard stop leaves `done` false.
		this.done = true
		if (failed > 0) this.error = `${failed} item${failed === 1 ? '' : 's'} failed to import.`
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
		this.#set('create', 'pending')
		this.done = false
		this.results = []
	}
}

function itemCount(e: ProjectExport): number {
	return e.scripts.length + e.flows.length + e.apps.length + e.resources.length
}
