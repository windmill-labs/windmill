import {
	appendTasks,
	deleteTask,
	listTasksForSession,
	putTask,
	type NewTask,
	type PersistedTask,
	type TaskStatus
} from './tasksDB'

export interface CreateTaskInput {
	subject: string
	description: string
	activeForm?: string
}

export interface UpdateTaskInput {
	subject?: string
	description?: string
	activeForm?: string
	status?: TaskStatus
}

/**
 * Reactive view of the active session's task list, owned by AIChatManager (like
 * SessionArtifactsStore). The consumer drives which session is loaded via setSession();
 * the tools mutate through createMany/update/remove, which persist and update the
 * in-memory list in one step.
 *
 * Ordered by `seq` — creation order is plan order, and the sequential tool loop can only
 * honor precedence expressed as ordering.
 */
export class SessionTasksStore {
	tasks = $state<PersistedTask[]>([])

	#sessionId: string | undefined
	// A later load always wins, even if an earlier DB read resolves after it.
	#loadToken = 0

	/** Load the given session's tasks into the reactive list, if it changed. */
	async setSession(sessionId: string | undefined): Promise<void> {
		// Skip same-id resyncs: in-memory owns the loaded session, so a DB reload would
		// drop tasks whose best-effort persist failed.
		if (sessionId === this.#sessionId) return
		this.#sessionId = sessionId
		await this.#load()
	}

	async #load(): Promise<void> {
		const token = ++this.#loadToken
		const id = this.#sessionId
		if (!id) {
			this.tasks = []
			return
		}
		const items = await listTasksForSession(id)
		if (token !== this.#loadToken) return
		this.tasks = sortBySeq(items)
	}

	// Bump the token so an in-flight #load, whose snapshot predates this write, cannot
	// clobber it.
	#applyWrite(next: PersistedTask[]): void {
		this.#loadToken++
		this.tasks = next
	}

	async listForSession(sessionId: string): Promise<PersistedTask[]> {
		if (sessionId === this.#sessionId) return [...this.tasks]
		return sortBySeq(await listTasksForSession(sessionId))
	}

	/**
	 * Every task currently being worked on. Plural on purpose: a test run detaches into
	 * the background after DETACH_AFTER_MS (or immediately, when the model asks), which
	 * frees the chat loop to work on the next task while the job is still running — so
	 * more than one task is genuinely in flight.
	 */
	get activeTasks(): PersistedTask[] {
		return this.tasks.filter((t) => t.status === 'in_progress')
	}

	/**
	 * Append tasks to `sessionId`'s plan. Batch because a plan created in one call is
	 * one transcript card rather than N. Sequence numbers are assigned by the store
	 * layer inside the write transaction (see appendTasks) so a second tab on the same
	 * session cannot allocate the same number and overwrite this write.
	 */
	async createMany(sessionId: string, inputs: CreateTaskInput[]): Promise<PersistedTask[]> {
		const now = Date.now()
		const drafts: NewTask[] = inputs.map((input) => ({
			subject: input.subject,
			description: input.description,
			activeForm: input.activeForm,
			status: 'pending' as TaskStatus,
			createdAt: now,
			updatedAt: now
		}))
		// Without IndexedDB the tasks live only in this store, so numbering from its own
		// list is both sufficient and the only option.
		let created = await appendTasks(sessionId, drafts)
		if (!created) {
			const existing = sessionId === this.#sessionId ? this.tasks : []
			let next = existing.reduce((max, t) => Math.max(max, t.seq), 0)
			created = drafts.map((draft) => ({ ...draft, sessionId, seq: ++next }))
		}
		if (sessionId === this.#sessionId) {
			this.#applyWrite(sortBySeq([...this.tasks, ...created]))
		}
		return created
	}

	/** Merge changes into one task. Returns undefined if the session has no such seq. */
	async update(
		sessionId: string,
		seq: number,
		input: UpdateTaskInput
	): Promise<PersistedTask | undefined> {
		const source = sessionId === this.#sessionId ? this.tasks : await listTasksForSession(sessionId)
		const existing = source.find((t) => t.seq === seq)
		if (!existing) return undefined
		const updated: PersistedTask = {
			...existing,
			subject: input.subject ?? existing.subject,
			description: input.description ?? existing.description,
			activeForm: input.activeForm ?? existing.activeForm,
			status: input.status ?? existing.status,
			updatedAt: Date.now()
		}
		await putTask(updated)
		if (sessionId === this.#sessionId) {
			this.#applyWrite(this.tasks.map((t) => (t.seq === seq ? updated : t)))
		}
		return updated
	}

	/** Drop one task. Returns false if the session has no such seq. */
	async remove(sessionId: string, seq: number): Promise<boolean> {
		const source = sessionId === this.#sessionId ? this.tasks : await listTasksForSession(sessionId)
		if (!source.some((t) => t.seq === seq)) return false
		await deleteTask(sessionId, seq)
		if (sessionId === this.#sessionId) {
			this.#applyWrite(this.tasks.filter((t) => t.seq !== seq))
		}
		return true
	}
}

function sortBySeq(items: PersistedTask[]): PersistedTask[] {
	return [...items].sort((a, b) => a.seq - b.seq)
}

/**
 * One-line state of the plan, returned on every write so the model tracks progress
 * without the tool echoing back the list it just authored.
 */
export function summarizeTasks(tasks: PersistedTask[]): string {
	if (tasks.length === 0) return 'No tasks.'
	const done = tasks.filter((t) => t.status === 'completed').length
	// Names every running task, not just the first — with backgrounded jobs several
	// can be in flight, and reporting one would tell the model its own plan is
	// narrower than it is.
	const active = tasks.filter((t) => t.status === 'in_progress').map((t) => t.subject)
	const now =
		active.length > 3 ? `${active.slice(0, 3).join(', ')} +${active.length - 3}` : active.join(', ')
	return `${done}/${tasks.length} done` + (now ? `, now: ${now}` : '')
}
